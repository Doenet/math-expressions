export function padNumberStringToDigits(numberString, padToDigits) {
  let nCharacters = Math.floor(padToDigits);
  if (nCharacters > 0) {
    if (numberString.includes(".")) {
      if(numberString[0] !== "0") {
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
    } else if(numberString.length < nCharacters) {
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
