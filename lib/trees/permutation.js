// MIT license'd code
// credit: http://stackoverflow.com/questions/9960908/permutations-in-javascript
export const anyPermutation = function (permutation, callback) {
  var length = permutation.length,
    c = Array(length).fill(0),
    i = 1;

  var result = callback(permutation);
  if (result) return result;

  while (i < length) {
    if (c[i] < i) {
      var k = i % 2 ? c[i] : 0,
        p = permutation[i];
      permutation[i] = permutation[k];
      permutation[k] = p;
      ++c[i];
      i = 1;

      result = callback(permutation);
      if (result) return result;
    } else {
      c[i] = 0;
      ++i;
    }
  }

  return false;
};
