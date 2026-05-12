var iterable = {};
var firstIterResult;

iterable[Symbol.iterator] = function() {
  var finalIterResult = { value: null, done: true };
  var nextIterResult = firstIterResult;

  return {
    next: function() {
      var iterResult = nextIterResult;
      nextIterResult = finalIterResult;
      return iterResult;
    }
  };
};

function run(label, value) {
  firstIterResult = value;
  var count = 0;
  try {
    for (var x of iterable) {
      count++;
      if (count > 3) {
        console.log(label, "too many");
        break;
      }
    }
    console.log(label, "ok", count);
  } catch (e) {
    console.log(label, "caught", count);
  }
}

run("true", true);
run("false", false);
run("string", "string");
run("undefined", undefined);
run("null", null);
run("four", 4);
run("nan", NaN);
run("symbol", Symbol("s"));
run("regexp", /regexp/);
run("object", {});
run("proxy-done-true", new Proxy({}, {
  get: function(receiver, name) {
    if (name === "done") return true;
    if (name === "value") return null;
    throw "bad";
  }
}));
run("proxy-done-false", new Proxy({}, {
  get: function(receiver, name) {
    if (name === "done") return false;
    if (name === "value") return 23;
    throw "bad";
  }
}));
