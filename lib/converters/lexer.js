// lexer class
//
// Processes input string to return tokens
//
// Token rules:
// array of rules to identify tokens
// Rules will be applied in order until a match is found.
// Each rule is an array of two or three elements
//   First element: a string to be converted into a regular expression
//   Second element: the token type
//   Third element (optional): replacement for actual string matched


class lexer {
  constructor(token_rules, whitespace='\\s') {

    this.input = '';
    this.location = 0;
    this.token_rules=[];

    // regular expression to identify whitespace at beginning
    this.initial_whitespace = new RegExp('^(' + whitespace + ')+');
    
    // convert first element of each rule to a regular expression that
    // starts at the beginning of the string
    for(let rule of token_rules) {
      this.token_rules.push([new RegExp('^'+rule[0])].concat(rule.slice(1)));
    }
  }
  
  set_input(input) {
    if(typeof input !== "string")
      throw new Error("Input must be a string");
    
    this.input = input;
    this.location = 0;
  }

  return_state() {
    return({ input: this.input, location: this.location });
  }

  set_state({ input=null, location=0 } = {}) {

    if(input !== null) {
      this.input = input;
      this.location = location;
    }
  }
  
  
  advance({ remove_initial_space=true } = {}) {
    // Find next token at beginning of input and delete from input.
    // Update location to be the position in original input corresponding
    // to end of match.
    // Return token, which is an array of token type and matched string


    let result = this.initial_whitespace.exec(this.input);
    if(result) {
      //first find any initial whitespace and adjust location
      let n_whitespace = result[0].length;
      this.input = this.input.slice(n_whitespace);
      this.location += n_whitespace;

      // don't remove initial space, return it as next token
      if(!remove_initial_space) {
	return { 
	  token_type: "SPACE",
	  token_text: result[0],
	  original_text: result[0],
	}
      }

      // otherwise ignore initial space and continue
    }
      
    // check for EOF
    if(this.input.length === 0) {
      return { 
	token_type: "EOF",
	token_text: "",
	original_text: "",
      }
    }
    
    // search through each token rule in order, finding first match
    result = null;
    
    for(var rule of this.token_rules) {
      result = rule[0].exec(this.input);
      
      if(result) {
	let n_characters = result[0].length;
	this.input = this.input.slice(n_characters);
	this.location += n_characters;
	break;
      }
    }

    // case that didn't find any matches
    if(result === null) {
      return { 
	token_type: "INVALID",
	token_text: this.input[0],
	original_text: this.input[0],
      }
    }

    // found a match, set token
    if(rule.length > 2) {
      // overwrite text by third element of rule
      return { token_type: rule[1],
	       token_text: rule[2],
	       original_text: result[0],
	     };
    }
    else {
      return { token_type: rule[1],
	       token_text: result[0],
	       original_text: result[0],
	     };
    }
  }

  
  unput(string) {
    // add string to beginning of input and adjust location
    
    if(typeof string !== "string")
      throw new Error("Input must be a string");

    this.location -= string.length;
    this.input = string + this.input;

  }
  
}

export default lexer;
