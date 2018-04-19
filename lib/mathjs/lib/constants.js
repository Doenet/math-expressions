import object from 'mathjs/lib/utils/object';
import bigConstants from 'mathjs/lib/utils/bignumber/constants';

function factory (type, config, load, typed, math) {
  // listen for changed in the configuration, automatically reload
  // constants when needed
  math.on('config', function (curr, prev) {
    if (curr.number !== prev.number || curr.define_pi !== prev.define_pi
	|| curr.define_e !== prev.define_e || curr.define_i !== prev.define_i) {
      factory(type, config, load, typed, math);
    }
  });

  setConstant(math, 'true', true);
  setConstant(math, 'false', false);
  setConstant(math, 'null', null);
  setConstant(math, 'uninitialized', 'Error: Constant uninitialized is removed since v4.0.0. Use null instead');

  if (config.number === 'BigNumber') {
    setConstant(math, 'Infinity', new type.BigNumber(Infinity));
    setConstant(math, 'NaN', new type.BigNumber(NaN));

    if(config.define_pi === false)
      deleteConstant(math, 'pi');
    else
      setLazyConstant(math, 'pi',  function () {return bigConstants.pi(type.BigNumber)});
    if(config.define_e === false)
      deleteConstant(math, 'e');
    else
      setLazyConstant(math, 'e',   function () {return bigConstants.e(type.BigNumber)});
  }
  else {
    setConstant(math, 'Infinity', Infinity);
    setConstant(math, 'NaN',      NaN);

    if(config.define_pi === false)
      deleteConstant(math, 'pi');
    else
      setConstant(math, 'pi',  Math.PI);
    if(config.define_e === false)
      deleteConstant(math, 'e');
    else
      setConstant(math, 'e',   Math.E);
  }

  if(config.define_i===false)
    deleteConstant(math, 'i');
  else
    // complex i
    setConstant(math, 'i', type.Complex.I);

}

// delete a constant in both math and mathWithTransform
function deleteConstant(math, name) {
    delete math[name];
    delete math.expression.mathWithTransform[name];
}

// create a constant in both math and mathWithTransform
function setConstant(math, name, value) {
  math[name] = value;
  math.expression.mathWithTransform[name] = value;
}

// create a lazy constant in both math and mathWithTransform
function setLazyConstant (math, name, resolver) {
  object.lazy(math, name,  resolver);
  object.lazy(math.expression.mathWithTransform, name,  resolver);
}

const lazy = false;  // no lazy loading of constants, the constants themselves are lazy when needed
const math = true;   // request access to the math namespace

export default { factory, lazy, math };
