export function padNumberStringToDigits(numberString, padToDigits) {
  let nCharacters = Math.floor(padToDigits);
  if (nCharacters > 0) {
    if (numberString.includes(".")) {
      if (numberString[0] !== "0") {
        // add one character for the decimal point
        nCharacters++;
      } else {
        // add one character for each leading zero plus the decimal point
        nCharacters += numberString.match(/^0\.0*/)[0].length;
      }
      if (numberString.length < nCharacters) {
        let nToPad = nCharacters - numberString.length;
        numberString += "0".repeat(nToPad);
      }
    } else if (numberString.length < nCharacters) {
      let nToPad = nCharacters - numberString.length;
      numberString += "." + "0".repeat(nToPad);
    }
  }

  return numberString;
}

export function padNumberStringToDecimals(numberString, padToDecimals) {
  let nDecimals = Math.floor(padToDecimals);
  if (nDecimals > 0) {
    if (numberString.includes(".")) {
      let currentDecimals = numberString.match(/\.\d*$/)[0].length - 1;

      if (currentDecimals < nDecimals) {
        let nToPad = nDecimals - currentDecimals;
        numberString += "0".repeat(nToPad);
      }
    } else {
      numberString += "." + "0".repeat(nDecimals);
    }
  }

  return numberString;
}

export function padNumberStringToDigitsAndDecimals(
  numberString,
  padToDigits,
  padToDecimals,
) {
  let nCharacters = Math.floor(padToDigits);

  if (!(nCharacters > 0)) {
    return padNumberStringToDecimals(numberString, padToDecimals);
  }

  let nDecimals = Math.floor(padToDecimals);
  if (!(nDecimals > 0)) {
    return padNumberStringToDigits(numberString, padToDigits);
  }

  // have both positive nCharacters and nDecimals
  if (numberString.includes(".")) {
    if (numberString[0] !== "0") {
      // add one character for the decimal point
      nCharacters++;
    } else {
      // add one character for each leading zero plus the decimal point
      nCharacters += numberString.match(/^0\.0*/)[0].length;
    }

    let nToPad = 0;

    // padding from digits
    if (numberString.length < nCharacters) {
      nToPad = nCharacters - numberString.length;
    }

    // padding from decimals
    let currentDecimals = numberString.match(/\.\d*$/)[0].length - 1;
    if (currentDecimals < nDecimals) {
      nToPad = Math.max(nToPad, nDecimals - currentDecimals);
    }

    if (nToPad > 0) {
      numberString += "0".repeat(nToPad);
    }
  } else {
    let nToPad = 0;

    // padding from digits
    if (numberString.length < nCharacters) {
      nToPad = nCharacters - numberString.length;
    }

    // padding from decimals
    nToPad = Math.max(nToPad, nDecimals);

    numberString += "." + "0".repeat(nToPad);
  }

  return numberString;
}
