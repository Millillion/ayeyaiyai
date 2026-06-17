//! Generation-counter memoization for the hot static-resolution entry points.
//!
//! The compiler's static resolvers (`resolve_object_binding_from_expression`,
//! `resolve_function_binding_from_expression_with_context`,
//! `resolve_static_call_result_expression_with_context`) are pure functions of
//! (expression, current function context, static-semantics state). Pathological
//! inputs re-resolve the same expression hundreds of thousands of times while
//! the state is unchanged. This module caches their results keyed by a
//! structural hash of the expression + context + a global *generation counter*
//! that every mutation funnel of resolution-relevant state must bump.
//!
//! Soundness model:
//! - Results are cached and reused ONLY in a "canonical" resolution context:
//!   no recursion guard of any resolution-related kind is active (tracked via
//!   `ResolutionGuardScope`). The resolvers' cycle guards make results
//!   context-dependent (a nested re-resolution of an expression already on a
//!   guard stack conservatively returns `None`); restricting the cache to
//!   guard-free entry points makes the cached value a deterministic function
//!   of (expression, context, generation) alone.
//! - The materialize cache additionally refuses results whose computation hit
//!   ANY recursion-guard block, including self-cycles (`is_clean_strict`):
//!   materialized expressions feed back into resolution state, and a cached
//!   cycle-cut expansion re-expands by one level per round-trip on
//!   self-referential tracked values, growing them without bound.
//! - Every mutation of state the resolvers consult must bump the generation
//!   (see the funnel audit in the optimization notes). A bump invalidates the
//!   whole cache.
//! - `AYY_MEMO_VERIFY=1` recomputes every cache hit from scratch and panics on
//!   divergence.
//! - `AYY_MEMO_DISABLE=1` disables the cache entirely (for A/B timing).
//! - `AYY_MEMO_STATS=1` prints hit/miss/gated counters at the end of each
//!   program compilation.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};

use crate::ir::hir::{ArrayElement, CallArgument, Expression, ObjectEntry};

use super::{LocalFunctionBinding, ObjectValueBinding};

thread_local! {
    static STATIC_STATE_GENERATION: Cell<u64> = const { Cell::new(0) };
    static RESOLUTION_GUARD_DEPTH: Cell<usize> = const { Cell::new(0) };
    static OBJECT_BINDING_CACHE: RefCell<MemoCache<Option<ObjectValueBinding>>> =
        RefCell::new(MemoCache::new());
    static FUNCTION_BINDING_CACHE: RefCell<MemoCache<Option<LocalFunctionBinding>>> =
        RefCell::new(MemoCache::new());
    static STATIC_CALL_RESULT_CACHE: RefCell<MemoCache<StaticCallResultValue>> =
        RefCell::new(MemoCache::new());
    static MEMBER_CAPTURE_SLOTS_CACHE: RefCell<MemoCache<MemberCaptureSlotsValue>> =
        RefCell::new(MemoCache::new());
    static MEMO_STATS: RefCell<MemoStats> = RefCell::new(MemoStats::default());
}

type StaticCallResultValue = Option<(Expression, Option<String>)>;
type MemberCaptureSlotsValue = Option<BTreeMap<String, String>>;

thread_local! {
    static MATERIALIZE_CACHE: RefCell<MemoCache<Expression>> = RefCell::new(MemoCache::new());
}

const MEMO_CACHE_CAPACITY_LIMIT: usize = 1 << 20;

struct MemoCache<V> {
    generation: u64,
    entries: HashMap<u128, V>,
}

impl<V> MemoCache<V> {
    fn new() -> Self {
        Self {
            generation: 0,
            entries: HashMap::new(),
        }
    }

    fn sync_generation(&mut self) {
        let generation = static_state_generation();
        if self.generation != generation {
            self.generation = generation;
            self.entries.clear();
        }
    }

    fn lookup(&mut self, key: u128) -> Option<&V> {
        self.sync_generation();
        self.entries.get(&key)
    }

    fn store(&mut self, key: u128, value: V) {
        self.sync_generation();
        if self.entries.len() >= MEMO_CACHE_CAPACITY_LIMIT {
            self.entries.clear();
        }
        self.entries.insert(key, value);
    }
}

#[derive(Default)]
struct MemoStats {
    object_hits: u64,
    object_misses: u64,
    function_hits: u64,
    function_misses: u64,
    call_hits: u64,
    call_misses: u64,
    member_capture_hits: u64,
    member_capture_misses: u64,
    materialize_hits: u64,
    materialize_misses: u64,
    gated_lookups: u64,
    generation_bumps: u64,
}

/// Bumped by every mutation funnel of resolution-relevant compiler state.
#[inline]
pub(in crate::backend::direct_wasm) fn bump_static_state_generation() {
    STATIC_STATE_GENERATION.with(|generation| generation.set(generation.get().wrapping_add(1)));
    if memo_stats_enabled() {
        MEMO_STATS.with(|stats| stats.borrow_mut().generation_bumps += 1);
    }
}

#[inline]
pub(in crate::backend::direct_wasm) fn static_state_generation() -> u64 {
    STATIC_STATE_GENERATION.with(|generation| generation.get())
}

/// RAII scope held by every resolution recursion guard. While any scope is
/// alive the resolution context is non-canonical and the memo cache is
/// bypassed.
pub(in crate::backend::direct_wasm) struct ResolutionGuardScope {
    class: u8,
}

thread_local! {
    static GUARD_CLASS_DEPTHS: RefCell<[u32; 32]> = const { RefCell::new([0; 32]) };
    static GUARD_CLASS_GATED: RefCell<[u64; 32]> = const { RefCell::new([0; 32]) };
}

impl ResolutionGuardScope {
    #[inline]
    pub(in crate::backend::direct_wasm) fn enter_class(class: u8) -> Self {
        RESOLUTION_GUARD_DEPTH.with(|depth| depth.set(depth.get() + 1));
        if memo_stats_enabled() {
            GUARD_CLASS_DEPTHS.with(|depths| depths.borrow_mut()[class as usize] += 1);
        }
        Self { class }
    }
}

impl Drop for ResolutionGuardScope {
    #[inline]
    fn drop(&mut self) {
        RESOLUTION_GUARD_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
        if memo_stats_enabled() {
            GUARD_CLASS_DEPTHS.with(|depths| {
                let mut depths = depths.borrow_mut();
                depths[self.class as usize] = depths[self.class as usize].saturating_sub(1);
            });
        }
    }
}

#[inline]
pub(in crate::backend::direct_wasm) fn memo_enabled() -> bool {
    !crate::ayy_env_flag!("AYY_MEMO_DISABLE")
}

#[inline]
pub(in crate::backend::direct_wasm) fn memo_verify_enabled() -> bool {
    crate::ayy_env_flag!("AYY_MEMO_VERIFY")
}

#[inline]
fn memo_stats_enabled() -> bool {
    crate::ayy_env_flag!("AYY_MEMO_STATS")
}

/// Returns true when the cache may be consulted/populated for the current
/// resolution context.
#[inline]
pub(in crate::backend::direct_wasm) fn memo_context_is_cacheable() -> bool {
    memo_enabled()
}

thread_local! {
    /// Monotonic serial assigned to every recursion-guard entry; used to
    /// distinguish blocks against guards entered *within* a memoized
    /// computation (deterministic self-cycles, safe to cache) from blocks
    /// against guards entered *before* it (outer-context dependence, unsafe).
    static GUARD_ENTER_SERIAL: Cell<u64> = const { Cell::new(0) };
    /// Minimum conflict serial observed since the innermost open store token
    /// was captured (`u64::MAX` = no blocks).
    static GUARD_BLOCK_TAINT: Cell<u64> = const { Cell::new(u64::MAX) };
}

/// Allocates the serial for a recursion-guard entry. Guards that can report
/// exact conflicts store this alongside their entries.
#[inline]
pub(in crate::backend::direct_wasm) fn next_guard_serial() -> u64 {
    GUARD_ENTER_SERIAL.with(|serial| {
        let next = serial.get() + 1;
        serial.set(next);
        next
    })
}

/// Called from a recursion-guard refusal path when the serial of the
/// conflicting (already active) guard entry is known.
#[inline]
pub(in crate::backend::direct_wasm) fn note_resolution_guard_block_conflict(serial: u64) {
    GUARD_BLOCK_TAINT.with(|taint| {
        if serial < taint.get() {
            taint.set(serial);
        }
    });
}

/// Called from recursion-guard refusal paths that cannot identify the
/// conflicting entry (depth limits, coarse shape sets). Taints every open
/// memoization window.
#[inline]
pub(in crate::backend::direct_wasm) fn note_resolution_guard_block() {
    note_resolution_guard_block_conflict(0);
}

/// Token capturing the memo-relevant context at the start of a computation;
/// the result may be cached only if `is_clean` afterwards: no state mutation
/// and no recursion-guard block against a guard that was already active when
/// the computation started.
pub(in crate::backend::direct_wasm) struct MemoStoreToken {
    generation: u64,
    start_serial: u64,
    parent_taint: u64,
}

impl MemoStoreToken {
    #[inline]
    pub(in crate::backend::direct_wasm) fn capture() -> Self {
        Self {
            generation: static_state_generation(),
            start_serial: GUARD_ENTER_SERIAL.with(|serial| serial.get()),
            parent_taint: GUARD_BLOCK_TAINT.with(|taint| taint.replace(u64::MAX)),
        }
    }

    /// True when the computation since `capture` was a pure function of the
    /// captured static state: guard entries created within the window have
    /// serials greater than `start_serial`, so any conflict at or below it
    /// involved an outer guard.
    #[inline]
    pub(in crate::backend::direct_wasm) fn is_clean(&self) -> bool {
        static_state_generation() == self.generation
            && GUARD_BLOCK_TAINT.with(|taint| taint.get()) > self.start_serial
    }

    /// Stricter cleanliness for caches whose values are *expressions* that
    /// feed back into resolution state (the materialize cache): no
    /// recursion-guard block of any kind, including self-cycles. A cycle-cut
    /// expansion depends on where the cycle was entered; caching it lets
    /// self-referential tracked values (for example `this.#field` tracked as
    /// `this.#field - 2`) grow by one level per cache round-trip, amplifying
    /// without bound at a fixed generation (compile live-locks and
    /// `AYY_MEMO_VERIFY` divergences).
    #[inline]
    pub(in crate::backend::direct_wasm) fn is_clean_strict(&self) -> bool {
        static_state_generation() == self.generation
            && GUARD_BLOCK_TAINT.with(|taint| taint.get()) == u64::MAX
    }
}

impl Drop for MemoStoreToken {
    #[inline]
    fn drop(&mut self) {
        // Propagate the window's taint to the enclosing window.
        GUARD_BLOCK_TAINT.with(|taint| {
            let window_taint = taint.get();
            taint.set(self.parent_taint.min(window_taint));
        });
    }
}

// ---------------------------------------------------------------------------
// Structural expression hashing (128-bit, no allocation).
// ---------------------------------------------------------------------------

/// Maximum number of expression nodes hashed for a cache key. Pathological
/// inputs (for example self-referential tracked values that grow by one
/// level per re-materialization) produce expressions with enormous node
/// counts; hashing them on every lookup turns each resolver call into an
/// O(size) walk and live-locks compilation. Oversized expressions are simply
/// not cached.
const MEMO_KEY_NODE_BUDGET: usize = 4096;

pub(in crate::backend::direct_wasm) struct ExpressionHasher {
    a: u64,
    b: u64,
    budget: usize,
    overflowed: bool,
}

impl ExpressionHasher {
    #[inline]
    pub(in crate::backend::direct_wasm) fn new(seed: u64) -> Self {
        Self {
            a: 0x243f_6a88_85a3_08d3 ^ seed,
            b: 0x1319_8a2e_0370_7344 ^ seed.rotate_left(32),
            budget: usize::MAX,
            overflowed: false,
        }
    }

    #[inline]
    fn with_node_budget(seed: u64, budget: usize) -> Self {
        let mut hasher = Self::new(seed);
        hasher.budget = budget;
        hasher
    }

    #[inline]
    fn write_u64(&mut self, value: u64) {
        self.a = (self.a.rotate_left(5) ^ value).wrapping_mul(0x517c_c1b7_2722_0a95);
        self.b = (self.b.rotate_left(9) ^ value).wrapping_mul(0x2545_f491_4f6c_dd1d);
    }

    #[inline]
    fn write_str(&mut self, text: &str) {
        let bytes = text.as_bytes();
        self.write_u64(bytes.len() as u64);
        let mut chunks = bytes.chunks_exact(8);
        for chunk in &mut chunks {
            self.write_u64(u64::from_le_bytes(chunk.try_into().unwrap()));
        }
        let remainder = chunks.remainder();
        if !remainder.is_empty() {
            let mut tail = [0u8; 8];
            tail[..remainder.len()].copy_from_slice(remainder);
            self.write_u64(u64::from_le_bytes(tail));
        }
    }

    #[inline]
    fn write_tag(&mut self, tag: u64) {
        self.write_u64(tag);
    }

    fn write_optional_str(&mut self, text: Option<&str>) {
        match text {
            Some(text) => {
                self.write_tag(1);
                self.write_str(text);
            }
            None => self.write_tag(0),
        }
    }

    pub(in crate::backend::direct_wasm) fn write_expression(&mut self, expression: &Expression) {
        if self.budget == 0 {
            self.overflowed = true;
            return;
        }
        self.budget -= 1;
        match expression {
            Expression::Number(value) => {
                self.write_tag(1);
                self.write_u64(value.to_bits());
            }
            Expression::BigInt(text) => {
                self.write_tag(2);
                self.write_str(text);
            }
            Expression::String(text) => {
                self.write_tag(3);
                self.write_str(text);
            }
            Expression::Bool(value) => {
                self.write_tag(4);
                self.write_u64(*value as u64);
            }
            Expression::Null => self.write_tag(5),
            Expression::Undefined => self.write_tag(6),
            Expression::NewTarget => self.write_tag(7),
            Expression::Array(elements) => {
                self.write_tag(8);
                self.write_u64(elements.len() as u64);
                for element in elements {
                    match element {
                        ArrayElement::Expression(expression) => {
                            self.write_tag(1);
                            self.write_expression(expression);
                        }
                        ArrayElement::Spread(expression) => {
                            self.write_tag(2);
                            self.write_expression(expression);
                        }
                    }
                }
            }
            Expression::Object(entries) => {
                self.write_tag(9);
                self.write_u64(entries.len() as u64);
                for entry in entries {
                    match entry {
                        ObjectEntry::Data { key, value } => {
                            self.write_tag(1);
                            self.write_expression(key);
                            self.write_expression(value);
                        }
                        ObjectEntry::Getter { key, getter } => {
                            self.write_tag(2);
                            self.write_expression(key);
                            self.write_expression(getter);
                        }
                        ObjectEntry::Setter { key, setter } => {
                            self.write_tag(3);
                            self.write_expression(key);
                            self.write_expression(setter);
                        }
                        ObjectEntry::Spread(expression) => {
                            self.write_tag(4);
                            self.write_expression(expression);
                        }
                    }
                }
            }
            Expression::Identifier(name) => {
                self.write_tag(10);
                self.write_str(name);
            }
            Expression::This => self.write_tag(11),
            Expression::Sent => self.write_tag(12),
            Expression::Member { object, property } => {
                self.write_tag(13);
                self.write_expression(object);
                self.write_expression(property);
            }
            Expression::SuperMember { property } => {
                self.write_tag(14);
                self.write_expression(property);
            }
            Expression::Assign { name, value } => {
                self.write_tag(15);
                self.write_str(name);
                self.write_expression(value);
            }
            Expression::AssignMember {
                object,
                property,
                value,
            } => {
                self.write_tag(16);
                self.write_expression(object);
                self.write_expression(property);
                self.write_expression(value);
            }
            Expression::AssignSuperMember { property, value } => {
                self.write_tag(17);
                self.write_expression(property);
                self.write_expression(value);
            }
            Expression::Await(expression) => {
                self.write_tag(18);
                self.write_expression(expression);
            }
            Expression::EnumerateKeys(expression) => {
                self.write_tag(19);
                self.write_expression(expression);
            }
            Expression::GetIterator(expression) => {
                self.write_tag(20);
                self.write_expression(expression);
            }
            Expression::IteratorClose(expression) => {
                self.write_tag(21);
                self.write_expression(expression);
            }
            Expression::Unary { op, expression } => {
                self.write_tag(22);
                self.write_u64(*op as u64);
                self.write_expression(expression);
            }
            Expression::Binary { op, left, right } => {
                self.write_tag(23);
                self.write_u64(*op as u64);
                self.write_expression(left);
                self.write_expression(right);
            }
            Expression::Conditional {
                condition,
                then_expression,
                else_expression,
            } => {
                self.write_tag(24);
                self.write_expression(condition);
                self.write_expression(then_expression);
                self.write_expression(else_expression);
            }
            Expression::Sequence(expressions) => {
                self.write_tag(25);
                self.write_u64(expressions.len() as u64);
                for expression in expressions {
                    self.write_expression(expression);
                }
            }
            Expression::Call { callee, arguments } => {
                self.write_tag(26);
                self.write_expression(callee);
                self.write_call_arguments(arguments);
            }
            Expression::SuperCall { callee, arguments } => {
                self.write_tag(27);
                self.write_expression(callee);
                self.write_call_arguments(arguments);
            }
            Expression::New { callee, arguments } => {
                self.write_tag(28);
                self.write_expression(callee);
                self.write_call_arguments(arguments);
            }
            Expression::Update { name, op, prefix } => {
                self.write_tag(29);
                self.write_str(name);
                self.write_u64(*op as u64);
                self.write_u64(*prefix as u64);
            }
        }
    }

    pub(in crate::backend::direct_wasm) fn write_call_arguments(
        &mut self,
        arguments: &[CallArgument],
    ) {
        self.write_u64(arguments.len() as u64);
        for argument in arguments {
            match argument {
                CallArgument::Expression(expression) => {
                    self.write_tag(1);
                    self.write_expression(expression);
                }
                CallArgument::Spread(expression) => {
                    self.write_tag(2);
                    self.write_expression(expression);
                }
            }
        }
    }

    #[inline]
    fn finish(&self) -> u128 {
        ((self.a as u128) << 64) | (self.b as u128)
    }
}

/// Structural 128-bit hash of an expression, usable as a compact cache key
/// component in place of `format!("{expression:?}")`.
pub(in crate::backend::direct_wasm) fn expression_structural_hash(expression: &Expression) -> u128 {
    let mut hasher = ExpressionHasher::new(0x5712_0c7);
    hasher.write_expression(expression);
    hasher.finish()
}

// ---------------------------------------------------------------------------
// NaN-aware structural fingerprints used by AYY_MEMO_VERIFY. The derived
// `PartialEq` on `Expression` makes `Number(NaN) != Number(NaN)`, so verify
// comparisons hash values (f64 by bit pattern) instead.
// ---------------------------------------------------------------------------

impl ExpressionHasher {
    fn write_optional_expression(&mut self, expression: Option<&Expression>) {
        match expression {
            Some(expression) => {
                self.write_tag(1);
                self.write_expression(expression);
            }
            None => self.write_tag(0),
        }
    }

    fn write_property_descriptor(&mut self, descriptor: &super::PropertyDescriptorBinding) {
        self.write_optional_expression(descriptor.value.as_ref());
        self.write_u64(descriptor.configurable as u64);
        self.write_u64(descriptor.enumerable as u64);
        self.write_u64(match descriptor.writable {
            None => 2,
            Some(writable) => writable as u64,
        });
        self.write_optional_expression(descriptor.getter.as_ref());
        self.write_optional_expression(descriptor.setter.as_ref());
        self.write_u64(descriptor.has_get as u64);
        self.write_u64(descriptor.has_set as u64);
    }

    fn write_object_binding(&mut self, binding: &ObjectValueBinding) {
        self.write_u64(binding.string_properties.len() as u64);
        for (name, value) in &binding.string_properties {
            self.write_str(name);
            self.write_expression(value);
        }
        self.write_u64(binding.symbol_properties.len() as u64);
        for (key, value) in &binding.symbol_properties {
            self.write_expression(key);
            self.write_expression(value);
        }
        self.write_u64(binding.property_descriptors.len() as u64);
        for (key, descriptor) in &binding.property_descriptors {
            self.write_expression(key);
            self.write_property_descriptor(descriptor);
        }
        self.write_u64(binding.non_enumerable_string_properties.len() as u64);
        for name in &binding.non_enumerable_string_properties {
            self.write_str(name);
        }
        self.write_u64(binding.runtime_symbol_properties as u64);
        self.write_u64(binding.extensible as u64);
    }
}

pub(in crate::backend::direct_wasm) fn verify_expressions_match(
    left: &Expression,
    right: &Expression,
) -> bool {
    expression_structural_hash(left) == expression_structural_hash(right)
}

pub(in crate::backend::direct_wasm) fn verify_object_bindings_match(
    left: &Option<ObjectValueBinding>,
    right: &Option<ObjectValueBinding>,
) -> bool {
    let fingerprint = |binding: &Option<ObjectValueBinding>| {
        let mut hasher = ExpressionHasher::new(0x0b1ec7_f1);
        match binding {
            None => hasher.write_tag(0),
            Some(binding) => {
                hasher.write_tag(1);
                hasher.write_object_binding(binding);
            }
        }
        hasher.finish()
    };
    fingerprint(left) == fingerprint(right)
}

pub(in crate::backend::direct_wasm) fn verify_static_call_results_match(
    left: &Option<(Expression, Option<String>)>,
    right: &Option<(Expression, Option<String>)>,
) -> bool {
    let fingerprint = |result: &Option<(Expression, Option<String>)>| {
        let mut hasher = ExpressionHasher::new(0xca11_f1);
        match result {
            None => hasher.write_tag(0),
            Some((value, label)) => {
                hasher.write_tag(1);
                hasher.write_expression(value);
                hasher.write_optional_str(label.as_deref());
            }
        }
        hasher.finish()
    };
    fingerprint(left) == fingerprint(right)
}

/// Returns `None` when the expression exceeds the node budget; such
/// expressions are not worth caching (hashing them per lookup is itself the
/// pathology the cache exists to avoid).
fn expression_context_key(
    seed: u64,
    expression: &Expression,
    arguments: Option<&[CallArgument]>,
    current_function_name: Option<&str>,
) -> Option<u128> {
    let mut hasher = ExpressionHasher::with_node_budget(seed, MEMO_KEY_NODE_BUDGET);
    hasher.write_optional_str(current_function_name);
    hasher.write_expression(expression);
    if let Some(arguments) = arguments {
        hasher.write_call_arguments(arguments);
    }
    (!hasher.overflowed).then(|| hasher.finish())
}

// ---------------------------------------------------------------------------
// Cache entry points (one per memoized resolver).
// ---------------------------------------------------------------------------

pub(in crate::backend::direct_wasm) fn object_binding_cache_key(
    expression: &Expression,
    current_function_name: Option<&str>,
) -> Option<u128> {
    expression_context_key(0x0b1ec7, expression, None, current_function_name)
}

pub(in crate::backend::direct_wasm) fn lookup_object_binding(
    key: u128,
) -> Option<Option<ObjectValueBinding>> {
    OBJECT_BINDING_CACHE.with(|cache| {
        let result = cache.borrow_mut().lookup(key).cloned();
        if memo_stats_enabled() {
            MEMO_STATS.with(|stats| {
                let mut stats = stats.borrow_mut();
                match result.is_some() {
                    true => stats.object_hits += 1,
                    false => stats.object_misses += 1,
                }
            });
        }
        result
    })
}

pub(in crate::backend::direct_wasm) fn store_object_binding(
    key: u128,
    value: Option<ObjectValueBinding>,
) {
    OBJECT_BINDING_CACHE.with(|cache| cache.borrow_mut().store(key, value));
}

pub(in crate::backend::direct_wasm) fn function_binding_cache_key(
    expression: &Expression,
    current_function_name: Option<&str>,
) -> Option<u128> {
    expression_context_key(0xf41c, expression, None, current_function_name)
}

pub(in crate::backend::direct_wasm) fn lookup_function_binding(
    key: u128,
) -> Option<Option<LocalFunctionBinding>> {
    FUNCTION_BINDING_CACHE.with(|cache| {
        let result = cache.borrow_mut().lookup(key).cloned();
        if memo_stats_enabled() {
            MEMO_STATS.with(|stats| {
                let mut stats = stats.borrow_mut();
                match result.is_some() {
                    true => stats.function_hits += 1,
                    false => stats.function_misses += 1,
                }
            });
        }
        result
    })
}

pub(in crate::backend::direct_wasm) fn store_function_binding(
    key: u128,
    value: Option<LocalFunctionBinding>,
) {
    FUNCTION_BINDING_CACHE.with(|cache| cache.borrow_mut().store(key, value));
}

pub(in crate::backend::direct_wasm) fn static_call_result_cache_key(
    callee: &Expression,
    arguments: &[CallArgument],
    current_function_name: Option<&str>,
) -> Option<u128> {
    expression_context_key(0xca11, callee, Some(arguments), current_function_name)
}

pub(in crate::backend::direct_wasm) fn member_capture_slots_cache_key(
    object: &Expression,
    property: &Expression,
    current_function_name: Option<&str>,
) -> Option<u128> {
    let mut hasher = ExpressionHasher::with_node_budget(0xcaff_5107, MEMO_KEY_NODE_BUDGET);
    hasher.write_optional_str(current_function_name);
    hasher.write_expression(object);
    hasher.write_expression(property);
    (!hasher.overflowed).then(|| hasher.finish())
}

pub(in crate::backend::direct_wasm) fn lookup_static_call_result(
    key: u128,
) -> Option<StaticCallResultValue> {
    STATIC_CALL_RESULT_CACHE.with(|cache| {
        let result = cache.borrow_mut().lookup(key).cloned();
        if memo_stats_enabled() {
            MEMO_STATS.with(|stats| {
                let mut stats = stats.borrow_mut();
                match result.is_some() {
                    true => stats.call_hits += 1,
                    false => stats.call_misses += 1,
                }
            });
        }
        result
    })
}

pub(in crate::backend::direct_wasm) fn store_static_call_result(
    key: u128,
    value: StaticCallResultValue,
) {
    STATIC_CALL_RESULT_CACHE.with(|cache| cache.borrow_mut().store(key, value));
}

pub(in crate::backend::direct_wasm) fn lookup_member_capture_slots(
    key: u128,
) -> Option<MemberCaptureSlotsValue> {
    MEMBER_CAPTURE_SLOTS_CACHE.with(|cache| {
        let result = cache.borrow_mut().lookup(key).cloned();
        if memo_stats_enabled() {
            MEMO_STATS.with(|stats| {
                let mut stats = stats.borrow_mut();
                match result.is_some() {
                    true => stats.member_capture_hits += 1,
                    false => stats.member_capture_misses += 1,
                }
            });
        }
        result
    })
}

pub(in crate::backend::direct_wasm) fn store_member_capture_slots(
    key: u128,
    value: MemberCaptureSlotsValue,
) {
    MEMBER_CAPTURE_SLOTS_CACHE.with(|cache| cache.borrow_mut().store(key, value));
}

pub(in crate::backend::direct_wasm) fn materialize_cache_key(
    expression: &Expression,
    current_function_name: Option<&str>,
) -> Option<u128> {
    expression_context_key(0x3a7e, expression, None, current_function_name)
}

pub(in crate::backend::direct_wasm) fn lookup_materialized_expression(
    key: u128,
) -> Option<Expression> {
    MATERIALIZE_CACHE.with(|cache| {
        let result = cache.borrow_mut().lookup(key).cloned();
        if memo_stats_enabled() {
            MEMO_STATS.with(|stats| {
                let mut stats = stats.borrow_mut();
                match result.is_some() {
                    true => stats.materialize_hits += 1,
                    false => stats.materialize_misses += 1,
                }
            });
        }
        result
    })
}

pub(in crate::backend::direct_wasm) fn store_materialized_expression(key: u128, value: Expression) {
    MATERIALIZE_CACHE.with(|cache| cache.borrow_mut().store(key, value));
}

pub(in crate::backend::direct_wasm) fn reset_memo_state() {
    bump_static_state_generation();
    OBJECT_BINDING_CACHE.with(|cache| cache.borrow_mut().entries.clear());
    FUNCTION_BINDING_CACHE.with(|cache| cache.borrow_mut().entries.clear());
    STATIC_CALL_RESULT_CACHE.with(|cache| cache.borrow_mut().entries.clear());
    MEMBER_CAPTURE_SLOTS_CACHE.with(|cache| cache.borrow_mut().entries.clear());
    MATERIALIZE_CACHE.with(|cache| cache.borrow_mut().entries.clear());
}

pub(in crate::backend::direct_wasm) fn dump_memo_stats(label: &str) {
    if !memo_stats_enabled() {
        return;
    }
    MEMO_STATS.with(|stats| {
        let stats = stats.borrow();
        eprintln!(
            "memo_stats:{label} object={}h/{}m function={}h/{}m call={}h/{}m member_capture={}h/{}m materialize={}h/{}m gated={} bumps={}",
            stats.object_hits,
            stats.object_misses,
            stats.function_hits,
            stats.function_misses,
            stats.call_hits,
            stats.call_misses,
            stats.member_capture_hits,
            stats.member_capture_misses,
            stats.materialize_hits,
            stats.materialize_misses,
            stats.gated_lookups,
            stats.generation_bumps,
        );
    });
    GUARD_CLASS_GATED.with(|gated| {
        let gated = gated.borrow();
        let mut entries = gated
            .iter()
            .enumerate()
            .filter(|(_, count)| **count > 0)
            .map(|(class, count)| format!("class{class}={count}"))
            .collect::<Vec<_>>();
        entries.sort();
        eprintln!("memo_stats:{label} gated_by: {}", entries.join(" "));
    });
}
