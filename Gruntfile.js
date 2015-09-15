
module.exports = function(grunt) {
    grunt.initConfig({
        pkg: grunt.file.readJSON('package.json'),

	urequire: {
	    _all: {
		clean: true,
	    },

	    lib: {
		main: 'math-expressions',		
		path: 'lib',
		dstPath: 'build/math-expressions.js',
		template: 'combined',

		bundle: {
		    dependencies: {
			rootExports: {
			    "math-expressions": "MathExpression"
			}
		    }
		}
	    },
	}
    });

    grunt.loadNpmTasks('grunt-urequire');
    grunt.registerTask('default',  ['urequire']);
};
