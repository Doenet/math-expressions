// I would need var parseString = require('../node_modules/xml-parser/index.js'); for urequire?
var parseString = require('xml-parser');

// This is an awfully weak MathML parser, but it's good enough for what MathJax generates
function parse(mml) {
    // math identifier
    if (mml.name == 'mi') {
	if (mml.content.length > 1) {
	    return "\\" + mml.content;
	} else {
	    return mml.content;
	}
    } 
    // math number
    else if (mml.name == 'mn') {
	    return mml.content;
    }
    // superscript
    else if (mml.name == 'msup') {
	return parse( mml.children[0] ) + '^{' + parse( mml.children[1] ) + "}";
    }
    // root
    else if (mml.name == 'mroot') {
	return "\\sqrt[" + parse( mml.children[1] ) + ']{' + parse( mml.children[1] ) + "}";
    }
    else if (mml.name == 'mfrac') {
	return "\\frac{" + parse( mml.children[0] ) + '}{' + parse( mml.children[1] ) + "}";
    }        
    // superscript
    else if (mml.name == 'msqrt') {
	return "\\sqrt{" + mml.children.map( parse ).join('') + "}";
    }    
    // math operator
    else if (mml.name == 'mo') {
	if (mml.content == '&#x2061;') { 
	    return ' ';
	} else {
	    return mml.content;
	}
    }
    else if ((mml.name == "mrow") && (mml.attributes.class == "MJX-TeXAtom-ORD")) {
	return mml.children.map( parse ).join('');
    } else if ((mml.name == 'math') || (mml.name == 'mrow')) {
	return '(' + mml.children.map( parse ).join('') + ')';
    }
}

exports.mmlToLatex = function(xml) {
    var result =  parse( parseString(xml).root );
    console.log( "parsed =", JSON.stringify(result) );
    return result;
};

