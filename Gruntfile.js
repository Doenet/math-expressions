
module.exports = function(grunt) {
    grunt.initConfig({
        pkg: grunt.file.readJSON('package.json'),

	urequire: {
	    _all: {
		clean: true,
	    },

	    lib: {
		main: 'lib/math-expressions',		
		path: '.',
		dstPath: 'build/math-expressions.js',
		template: 'combined',

		bundle: {
		    filez: [
			"lib/**",
			"lib/debug",
			"node_modules/xml-parser/index.js",
			"node_modules/number-theory/index.js",						
			"node_modules/number-theory/lib/**",			
		    ],
		    dependencies: {
			replace: {
			    "lib/debug": "debug",
			    "node_modules/number-theory/index.js": "number-theory"
			},
			imports: {
			    debug: "debug"
			},
			rootExports: {
			    "math-expressions": "MathExpression"
			}
		    }
		}
	    }
	}
    });

    grunt.loadNpmTasks('grunt-urequire');
    grunt.registerTask('default',  ['urequire']);
};
