const humane_errs = [];

const humane = {
  assert_eq: (left, right) => {
    if (left !== right) {
      humane_errs.push(
        `Equality Assertion failed. Left: ${JSON.stringify(
          left
        )}, Right: ${JSON.stringify(right)}`
      );
    }
  },
  waitFor: async (q, timeout = 4000) => {
    let start = Date.now();
    const throttle = 50; // TODO: configure

    let r = await q();
    while (!r) {
      await new Promise((r) => setTimeout(r, throttle));
      r = await q();
      if (Date.now() - start > timeout) {
        break;
      }
    }
    if (!r) {
      throw new Error(
        `:humane_err: waitFor timed out at ${timeout}ms, no result for "${q.toString()}"`
      );
    }
    return r;
  },
  querySelector: async (s, timeout = 4000) => {
    try {
      return await humane.waitFor(() => document.querySelector(s), timeout);
    } catch (e) {
      if (/:humane_err:/.test(e.toString())) {
        throw new Error(
          `:humane_err: querySelector timed out at ${timeout}ms, no elements matching "${s}"`
        );
      } else {
        throw e;
      }
    }
  },
  querySelectorAll: async (s, timeout = 4000) => {
    try {
      return await humane.waitFor(() => {
        let els = document.querySelectorAll(s);
        if (!els?.length) return null;
        return els;
      }, timeout);
    } catch (e) {
      if (/:humane_err:/.test(e.toString())) {
        throw new Error(
          `:humane_err: querySelectorAll timed out at ${timeout}ms, no elements matching "${s}"`
        );
      } else {
        throw e;
      }
    }
  },
};

const inner = async () => {
  // insert_humane_inner_js
};

let inner_response;
try {
  inner_response = await inner();
} catch (e) {
  let errString = e.toString();
  if (/:humane_err:/.test(errString)) {
    humane_errs.push(errString.replace(/:humane_err: ?/, ""));
  } else {
    humane_errs.push(`JavaScript error: ${errString}`);
  }
}

return { humane_errs, inner_response };
