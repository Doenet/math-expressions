function ParseError(message, location) {
    this.name = 'ParseError';
    this.message = message || 'Error parsing input';
    this.stack = (new Error()).stack;
    this.location = location;
}
ParseError.prototype = Object.create(Error.prototype);
ParseError.prototype.constructor = ParseError;

exports.ParseError = ParseError;
