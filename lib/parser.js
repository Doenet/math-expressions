var _ = require('underscore');

kinds = ['text', 'latex', 'ast'];

// define the basic converters
converters = {
    latex: {
	to: { 
	    //ast: latexToAst,
	}
    },
    text: {
	to: { 
	    ast: require('./text-to-ast').textToAst,
	}
    },
    ast: {
	to: {
	    text: require('./ast-to-text').astToText,
	    //latex: astToLatex,
	}
    }
};

// compute the transitive closure
var foundNew = true;

while( foundNew ) {
    foundNew = false;
    
    _.each( kinds, function(a) {
	if (a in converters) {
	    _.each( kinds, function(b) {
		if ((b in converters) && (b in converters[a].to)) {
		    _.each( kinds, function(c) {
			if ((c in converters[b].to) && (!(c in converters[a].to))) {
			    foundNew = true;
			    converters[a].to[c] = _.compose( converters[b].to[c], converters[a].to[b] );
			}
		    });
		}
	    });
	}
    });
}

// export the converters
_.each( kinds, function(a) {
    exports[a] = converters[a];
});
