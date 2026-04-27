/**
 * Expand a number string in scientific notation (for example, "1.23e-4")
 * into a plain decimal string without exponent syntax.
 *
 * This utility is used by output converters when callers request that
 * scientific notation be avoided while preserving deterministic formatting.
 */
export function expandScientificNotation(numberString) {
  let eIndex = numberString.indexOf("e");
  if (eIndex === -1) {
    return numberString;
  }

  let mantissa = numberString.substring(0, eIndex);
  let exponent = Number(numberString.substring(eIndex + 1));

  let sign = "";
  if (mantissa[0] === "-" || mantissa[0] === "+") {
    sign = mantissa[0] === "-" ? "-" : "";
    mantissa = mantissa.substring(1);
  }

  let decimalIndex = mantissa.indexOf(".");
  if (decimalIndex === -1) {
    decimalIndex = mantissa.length;
  }

  let digits = mantissa.replace(".", "");
  let shiftedDecimalIndex = decimalIndex + exponent;

  if (shiftedDecimalIndex <= 0) {
    return sign + "0." + "0".repeat(-shiftedDecimalIndex) + digits;
  }

  if (shiftedDecimalIndex >= digits.length) {
    return sign + digits + "0".repeat(shiftedDecimalIndex - digits.length);
  }

  return (
    sign +
    digits.substring(0, shiftedDecimalIndex) +
    "." +
    digits.substring(shiftedDecimalIndex)
  );
}
