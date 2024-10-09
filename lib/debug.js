export default function (name) {
  return function (message) {
    console.log(name + ": ", message);
  };
}
