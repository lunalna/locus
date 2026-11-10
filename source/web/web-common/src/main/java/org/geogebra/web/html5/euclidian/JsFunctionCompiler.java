package org.geogebra.web.html5.euclidian;

import org.geogebra.common.kernel.Kernel;
import org.geogebra.common.kernel.StringTemplate;
import org.geogebra.common.kernel.arithmetic.BooleanValue;
import org.geogebra.common.kernel.arithmetic.Evaluatable;
import org.geogebra.common.kernel.arithmetic.ExpressionNode;
import org.geogebra.common.kernel.arithmetic.ExpressionValue;
import org.geogebra.common.kernel.arithmetic.Function;
import org.geogebra.common.kernel.arithmetic.FunctionVariable;
import org.geogebra.common.kernel.arithmetic.Inspecting;
import org.geogebra.common.kernel.arithmetic.MinusOne;
import org.geogebra.common.kernel.arithmetic.MyList;
import org.geogebra.common.kernel.arithmetic.MyNumberPair;
import org.geogebra.common.kernel.arithmetic.NumberValue;
import org.geogebra.common.kernel.geos.GeoFunction;
import org.geogebra.common.plugin.Operation;
import org.geogebra.common.util.DoubleUtil;

import jsinterop.annotations.JsFunction;
import jsinterop.base.Js;

/**
 * Compiles functions of x to JavaScript, so the browser's JIT evaluates them instead of the
 * expression interpreter. Parts that do not depend on x are evaluated once while compiling.
 */
public final class JsFunctionCompiler {

	// booleans are 1, 0 or NaN when undefined; comparisons and sgn use the kernel's tolerance
	private static final String HELPERS = "var E=" + Kernel.STANDARD_PRECISION + ";"
			+ "function d(a,b){return isFinite(a)&&isFinite(b)}"
			+ "function gt(a,b){return d(a,b)?(a>b+E?1:0):NaN}"
			+ "function ge(a,b){return d(a,b)?(a+E>b?1:0):NaN}"
			+ "function sgn(a){return a!==a?NaN:Math.abs(a)<E?0:a>0?1:-1}"
			// a negative base to the power p/q, like ExpressionNodeEvaluator.negPower
			+ "function root(b,p,q){var a=p===1?b:Math.pow(b,p);if(a>0)return Math.pow(a,1/q);"
			+ "return Math.abs(q)%2===1?-Math.pow(-a,1/q):NaN}";

	private static final RuntimeException UNSUPPORTED = new RuntimeException();

	private FunctionVariable variable;
	private String x = "x";
	private int temps;

	@JsFunction
	private interface JsRealFunction {
		double apply(double x);
	}

	private JsFunctionCompiler(FunctionVariable variable) {
		this.variable = variable;
	}

	/**
	 * @param function function of x
	 * @return compiled function, or null if it uses something the compiler does not support
	 */
	public static Evaluatable compile(GeoFunction function) {
		Function fun = function.getFunction();
		if (!isPlain(function)) {
			return null;
		}
		JsFunctionCompiler compiler = new JsFunctionCompiler(fun.getFunctionVariable());
		String expression;
		try {
			expression = compiler.code(fun.getExpression());
		} catch (RuntimeException e) {
			return null;
		}
		StringBuilder source = new StringBuilder(HELPERS).append("return function(x){");
		for (int i = 0; i < compiler.temps; i++) {
			source.append(i == 0 ? "var " : ",").append('t').append(i);
		}
		source.append(compiler.temps > 0 ? ";" : "").append("return ").append(expression)
				.append("}");
		JsRealFunction compiled = Js.uncheckedCast(
				new elemental2.core.Function(source.toString()).call());
		return compiled::apply;
	}

	// subclasses of Function evaluate something else than their expression
	private static boolean isPlain(GeoFunction function) {
		Function fun = function.getFunction();
		return function.isDefined() && fun != null && fun.getClass() == Function.class
				&& !fun.isBooleanFunction();
	}

	private String code(ExpressionValue value) {
		ExpressionValue ev = value.unwrap();
		if (ev == variable) {
			return x;
		}
		if (!ev.any(Inspecting::isFunctionVariable)) {
			return constant(ev);
		}
		if (!(ev instanceof ExpressionNode)) {
			throw UNSUPPORTED;
		}
		ExpressionNode node = (ExpressionNode) ev;
		ExpressionValue left = node.getLeft();
		ExpressionValue right = node.getRight();
		switch (node.getOperation()) {
		case PLUS:
			return binary(left, "+", right);
		case MINUS:
			return binary(left, "-", right);
		case MULTIPLY:
			return binary(left, "*", right);
		case DIVIDE:
			return binary(left, "/", right);
		case POWER:
			return power(left, right);
		case SQRT:
		case SQRT_SHORT:
			return call("Math.sqrt", left);
		case CBRT:
			return call("Math.cbrt", left);
		case ABS:
			return call("Math.abs", left);
		case SGN:
			return call("sgn", left);
		case EXP:
			return call("Math.exp", left);
		case LOG:
			return call("Math.log", left);
		case LOG10:
			return call("Math.log10", left);
		case LOG2:
			return call("Math.log2", left);
		case SIN:
			return call("Math.sin", left);
		case COS:
			return call("Math.cos", left);
		case TAN:
			return call("Math.tan", left);
		case SINH:
			return call("Math.sinh", left);
		case COSH:
			return call("Math.cosh", left);
		case TANH:
			return call("Math.tanh", left);
		case ARCTAN:
			return call("Math.atan", left);
		case LESS:
			return "gt(" + code(right) + "," + code(left) + ")";
		case LESS_EQUAL:
			return "ge(" + code(right) + "," + code(left) + ")";
		case GREATER:
			return "gt(" + code(left) + "," + code(right) + ")";
		case GREATER_EQUAL:
			return "ge(" + code(left) + "," + code(right) + ")";
		case AND:
		case AND_INTERVAL:
			return "Math.min(" + code(left) + "," + code(right) + ")";
		case OR:
			return "Math.max(" + code(left) + "," + code(right) + ")";
		case NOT:
			return "(1-" + code(left) + ")";
		case IF:
		case IF_SHORT:
			return "(" + code(left) + "===1?" + code(right) + ":NaN)";
		case IF_ELSE:
			return ifElse(left, right);
		case IF_LIST:
			return ifList(left, right);
		case FUNCTION:
			return function(left.unwrap(), right);
		default:
			throw UNSUPPORTED;
		}
	}

	private String constant(ExpressionValue ev) {
		ExpressionValue result = ev.evaluate(StringTemplate.defaultTemplate);
		if (result instanceof BooleanValue) {
			BooleanValue bool = (BooleanValue) result;
			return bool.isDefined() ? (bool.getBoolean() ? "1" : "0") : "NaN";
		}
		if (result instanceof NumberValue) {
			return "(" + result.evaluateDouble() + ")";
		}
		throw UNSUPPORTED;
	}

	private String binary(ExpressionValue left, String operator, ExpressionValue right) {
		return "(" + code(left) + operator + code(right) + ")";
	}

	private String call(String name, ExpressionValue argument) {
		return name + "(" + code(argument) + ")";
	}

	private String temp() {
		return "t" + temps++;
	}

	// follows ExpressionNodeEvaluator.handlePower
	private String power(ExpressionValue left, ExpressionValue right) {
		if (!left.any(Inspecting::isFunctionVariable) && left.evaluateDouble() == Math.E) {
			return call("Math.exp", right);
		}
		String base = code(left);
		String exponent = code(right);
		ExpressionNode fraction = null;
		boolean inverse = false;
		if (right instanceof ExpressionNode) {
			ExpressionNode node = (ExpressionNode) right;
			if (node.isOperation(Operation.DIVIDE)) {
				fraction = node;
			} else if (node.isOperation(Operation.MULTIPLY) && node.getLeft() instanceof MinusOne
					&& node.getRight().isOperation(Operation.DIVIDE)) {
				fraction = node.getRight().wrap();
				inverse = true;
			}
		}
		if (fraction == null || fraction.any(Inspecting::isFunctionVariable)) {
			return "Math.pow(" + base + "," + exponent + ")";
		}
		double p = fraction.getLeft().evaluateDouble();
		double q = fraction.getRight().evaluateDouble();
		long pl = Math.round(p);
		long ql = Math.round(q);
		if (!DoubleUtil.isEqual(p, pl) || (q != 0 && !DoubleUtil.isEqual(q, ql))) {
			return "Math.pow(" + base + "," + exponent + ")";
		}
		long gcd = Kernel.gcd(pl, ql);
		String t = temp();
		String negative = q == 0 || gcd == 0 ? "NaN"
				: (inverse ? "1/" : "") + "root(" + t + "," + pl / gcd + "," + ql / gcd + ")";
		return "((" + t + "=" + base + ")<0?" + negative + ":Math.pow(" + t + "," + exponent
				+ "))";
	}

	private String ifElse(ExpressionValue left, ExpressionValue right) {
		if (!(left instanceof MyNumberPair)) {
			throw UNSUPPORTED;
		}
		MyNumberPair pair = (MyNumberPair) left;
		String t = temp();
		return "((" + t + "=" + code(pair.getX()) + ")===1?" + code(pair.getY()) + ":" + t
				+ "===0?" + code(right) + ":NaN)";
	}

	private String ifList(ExpressionValue left, ExpressionValue right) {
		if (!(left instanceof MyList) || !(right instanceof MyList)) {
			throw UNSUPPORTED;
		}
		MyList conditions = (MyList) left;
		MyList values = (MyList) right;
		StringBuilder sb = new StringBuilder("(");
		for (int i = 0; i < conditions.size(); i++) {
			sb.append(code(conditions.get(i))).append("===1?").append(code(values.get(i)))
					.append(':');
		}
		sb.append(values.size() > conditions.size() ? code(values.get(conditions.size())) : "NaN");
		return sb.append(')').toString();
	}

	// inlines another function of one variable, with its argument in a temporary variable
	private String function(ExpressionValue left, ExpressionValue right) {
		if (!(left instanceof GeoFunction) || !isPlain((GeoFunction) left)) {
			throw UNSUPPORTED;
		}
		Function fun = ((GeoFunction) left).getFunction();
		String t = temp();
		String argument = code(right);
		FunctionVariable outerVariable = variable;
		String outerX = x;
		variable = fun.getFunctionVariable();
		x = t;
		String body = code(fun.getExpression());
		variable = outerVariable;
		x = outerX;
		return "(" + t + "=" + argument + "," + body + ")";
	}
}
