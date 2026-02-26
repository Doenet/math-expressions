import { get_tree } from "../trees/util.js";

function create_discrete_infinite_set({
  offsets,
  periods,
  min_index = ["-", Infinity],
  max_index = Infinity,
} = {}) {
  offsets = get_tree(offsets);
  periods = get_tree(periods);
  min_index = get_tree(min_index);
  max_index = get_tree(max_index);

  if (offsets === undefined || periods === undefined) return undefined;

  let results = [];
  if (offsets[0] === "list") {
    if (periods[0] === "list") {
      if (offsets.length !== periods.length || offsets.length === 1)
        return undefined;
      for (let i = 1; i < offsets.length; i++)
        results.push(["tuple", offsets[i], periods[i], min_index, max_index]);
    } else {
      for (let i = 1; i < offsets.length; i++)
        results.push(["tuple", offsets[i], periods, min_index, max_index]);
    }
  } else {
    results.push(["tuple", offsets, periods, min_index, max_index]);
  }
  return ["discrete_infinite_set"].concat(results);
}

export { create_discrete_infinite_set };
