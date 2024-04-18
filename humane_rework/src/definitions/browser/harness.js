const humane = new HumaneHarness();

const inner = async () => {
  // insert_humane_inner_js
};

let inner_response;

try {
  inner_response = await inner();
} catch (e) {
  let errString = e.toString();
  if (/:humane_err:/.test(errString)) {
    humane.errors.push(errString.replace(/:humane_err: ?/, ""));
  } else {
    humane.errors.push(`JavaScript error: ${errString}`);
  }
}

if (humane.errors.length) {
  return {
    humane_errs: humane.errors,
    inner_response,
    logs: humane_log_events["ALL"].join("\n"),
  };
} else {
  return { humane_errs: [], inner_response };
}
