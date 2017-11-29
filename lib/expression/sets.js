"use strict";

var get_tree = require('../trees/util').get_tree;

function create_discrete_infinite_set(offsets, periods) {

    offsets = get_tree(offsets);
    periods = get_tree(periods);

    if(offsets === undefined || periods === undefined)
	return undefined;
    
    let results = []
    if(offsets[0] === 'list') {
	if(periods[0] === 'list') {
	    if(offsets.length != periods.length || offsets.length == 1)
		return undefined;
	    for(let i=1; i<offsets.length; i++)
		results.push(['tuple', offsets[i], periods[i]]);
	}
	else {
	    for(let i=1; i<offsets.length; i++)
		results.push(['tuple', offsets[i], periods]);
	}
    }
    else {
	results.push(['tuple', offsets, periods]);

    }
    return ['discrete_infinite_set'].concat(results);
}

exports.create_discrete_infinite_set = create_discrete_infinite_set;
