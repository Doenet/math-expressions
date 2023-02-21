export function get_all_units() {
  return {
    '%': { scale: x => ["/", x, 100], only_scales: true },
    '$': { prefix: true, scale: x => x },
    'deg': { scale: x=> ["/", ["*", x, "pi"], 180], only_scales: true }
  }
}

let all_units = get_all_units();

export function get_unit_of_tree(tree) {
  if (tree[0] === "unit") {
    for (let unit in all_units) {
      if (all_units[unit].prefix) {
        if (tree[1] === unit) {
          return unit;
        }
      } else {
        if (tree[2] === unit) {
          return unit;
        }
      }
    }
  }

  return null;

}

export function get_unit_value_of_tree(tree) {
  if (tree[0] === "unit") {
    for (let unit in all_units) {
      if (all_units[unit].prefix) {
        if (tree[1] === unit) {
          return [unit, tree[2]];
        }
      } else {
        if (tree[2] === unit) {
          return [unit, tree[1]];
        }
      }
    }
  }

  return null;

}
