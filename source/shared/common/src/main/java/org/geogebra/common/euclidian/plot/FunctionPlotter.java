package org.geogebra.common.euclidian.plot;

import org.geogebra.common.awt.AwtFactory;
import org.geogebra.common.awt.GGeneralPath;
import org.geogebra.common.awt.GPoint;
import org.geogebra.common.euclidian.EuclidianView;
import org.geogebra.common.kernel.arithmetic.Evaluatable;
import org.geogebra.common.kernel.arithmetic.ExpressionNode;
import org.geogebra.common.kernel.arithmetic.ExpressionValue;
import org.geogebra.common.kernel.arithmetic.Function;
import org.geogebra.common.kernel.arithmetic.FunctionVariable;
import org.geogebra.common.kernel.arithmetic.Inspecting;
import org.geogebra.common.kernel.geos.GeoFunction;
import org.geogebra.common.plugin.Operation;

/**
 * Plots a function y = f(x) from samples on a grid aligned to world x, so panning and zooming
 * reuse earlier values. The path keeps at most the first, lowest, highest and last point of
 * each pixel column.
 */
public class FunctionPlotter {
	private static final int SAMPLES_PER_PIXEL = 2;
	private static final double JUMP_PIXELS = 10;
	private static final int SPLIT_BISECTIONS = 24;
	private static final double CONTINUITY_TOLERANCE = 1E-4;
	// keeps far-away points off screen, with room for the stroke width
	private static final double MARGIN = 20;
	private static final byte UNKNOWN = 0;
	private static final byte CONTINUOUS = 1;
	private static final byte SPLIT = 2;

	/** Whether small sign changes are searched for hidden poles, set from the performance settings. */
	public static boolean findHiddenPoles = true;

	/** Turns a function into a faster evaluator, or null; set by platforms that can compile. */
	public static java.util.function.Function<GeoFunction, Evaluatable> compiler =
			function -> null;

	private final GeoFunction function;
	private final EuclidianView view;
	private final GGeneralPath path = AwtFactory.getPrototype().newGeneralPath();
	// null until prepared for the current definition
	private Evaluatable evaluator;
	// outside of this the function is undefined because of its condition
	private double domainMin;
	private double domainMax;

	// sample i is at x = (first + i) * step
	private double step = Double.NaN;
	private double first;
	private int count;
	private double[] ys = new double[0];
	// what lies between sample i and i + 1; a SPLIT ends at (endX, endY) and resumes at
	// (startX, startY), where an undefined side has NaN as y
	private byte[] gaps = new byte[0];
	private double[] splitEndX = new double[0];
	private double[] splitEndY = new double[0];
	private double[] splitStartX = new double[0];
	private double[] splitStartY = new double[0];
	// the value the gap was tested for, a continuous curve between both sides has to reach it
	private double[] gapTarget = new double[0];

	private boolean built;
	private double builtXZero;
	private double builtYZero;
	private double builtXScale;
	private double builtYScale;
	private int builtWidth;
	private int builtHeight;
	private GPoint labelPoint;

	private boolean open;
	private double top;
	private double bottom;
	private int column;
	private double lastX;
	private double lastY;
	private double minX;
	private double minY;
	private double maxX;
	private double maxY;
	private boolean minFirst;
	private double endX;
	private double endY;

	/**
	 * @param function plotted function
	 * @param view view to plot in
	 */
	public FunctionPlotter(GeoFunction function, EuclidianView view) {
		this.function = function;
		this.view = view;
	}

	/**
	 * Drops all samples, call when the function changed.
	 */
	public void invalidate() {
		evaluator = null;
		step = Double.NaN;
		count = 0;
		built = false;
	}

	/**
	 * @return screen path for the current view
	 */
	public GGeneralPath getPath() {
		update();
		return path;
	}

	/**
	 * @return first visible point of the curve in screen coordinates, or null
	 */
	public GPoint getLabelPoint() {
		update();
		return labelPoint;
	}

	private void update() {
		if (built && builtXZero == view.getXZero() && builtYZero == view.getYZero()
				&& builtXScale == view.getXscale() && builtYScale == view.getYscale()
				&& builtWidth == view.getWidth() && builtHeight == view.getHeight()) {
			return;
		}
		built = true;
		builtXZero = view.getXZero();
		builtYZero = view.getYZero();
		builtXScale = view.getXscale();
		builtYScale = view.getYscale();
		builtWidth = view.getWidth();
		builtHeight = view.getHeight();
		if (evaluator == null) {
			prepare();
		}
		double min = Math.max(Math.max(view.getXmin(), function.getMinParameter()), domainMin);
		double max = Math.min(Math.min(view.getXmax(), function.getMaxParameter()), domainMax);
		path.reset();
		labelPoint = null;
		if (!(min < max)) {
			return;
		}
		sample(min, max);
		build(min, max);
	}

	private void prepare() {
		Evaluatable compiled = compiler.apply(function);
		evaluator = compiled != null ? compiled : function;
		domainMin = Double.NEGATIVE_INFINITY;
		domainMax = Double.POSITIVE_INFINITY;
		Function fun = function.getFunction();
		if (fun != null && (fun.getExpression().isOperation(Operation.IF)
				|| fun.getExpression().isOperation(Operation.IF_SHORT))) {
			restrictDomain(fun.getExpression().getLeft(), fun.getFunctionVariable());
		}
	}

	private void restrictDomain(ExpressionValue condition, FunctionVariable x) {
		if (!(condition instanceof ExpressionNode)) {
			return;
		}
		ExpressionNode node = (ExpressionNode) condition;
		ExpressionValue left = node.getLeft().unwrap();
		ExpressionValue right = node.getRight().unwrap();
		switch (node.getOperation()) {
		case AND:
		case AND_INTERVAL:
			restrictDomain(left, x);
			restrictDomain(right, x);
			break;
		case LESS:
		case LESS_EQUAL:
			restrictDomain(left, right, x);
			break;
		case GREATER:
		case GREATER_EQUAL:
			restrictDomain(right, left, x);
			break;
		default:
			break;
		}
	}

	// applies the condition low <= high when one side is x and the other a constant
	private void restrictDomain(ExpressionValue low, ExpressionValue high, FunctionVariable x) {
		if (low == x && !high.any(Inspecting::isFunctionVariable)) {
			domainMax = Math.min(domainMax, high.evaluateDouble());
		} else if (high == x && !low.any(Inspecting::isFunctionVariable)) {
			domainMin = Math.max(domainMin, low.evaluateDouble());
		}
	}

	private void sample(double min, double max) {
		double wanted = 1 / (SAMPLES_PER_PIXEL * view.getXscale());
		// a power of two keeps the grid fixed while zooming within a factor of two
		double newStep = Math.pow(2, Math.floor(Math.log(wanted) / Math.log(2)));
		double newFirst = Math.floor(min / newStep);
		int newCount = (int) (Math.ceil(max / newStep) - newFirst) + 1;
		if (newStep == step && newFirst == first && newCount == count) {
			return;
		}
		boolean reuse = newStep == step && newFirst < first + count && newFirst + newCount > first;
		int shift = reuse ? (int) (newFirst - first) : 0;
		double[] newYs = new double[newCount];
		byte[] newGaps = new byte[newCount];
		double[] newEndX = new double[newCount];
		double[] newEndY = new double[newCount];
		double[] newStartX = new double[newCount];
		double[] newStartY = new double[newCount];
		double[] newTarget = new double[newCount];
		for (int i = 0; i < newCount; i++) {
			int old = i + shift;
			if (reuse && old >= 0 && old < count) {
				newYs[i] = ys[old];
				if (old + 1 < count) {
					newGaps[i] = gaps[old];
					newEndX[i] = splitEndX[old];
					newEndY[i] = splitEndY[old];
					newStartX[i] = splitStartX[old];
					newStartY[i] = splitStartY[old];
					newTarget[i] = gapTarget[old];
				}
			} else {
				newYs[i] = evaluator.value((newFirst + i) * newStep);
			}
		}
		step = newStep;
		first = newFirst;
		count = newCount;
		ys = newYs;
		gaps = newGaps;
		splitEndX = newEndX;
		splitEndY = newEndY;
		splitStartX = newStartX;
		splitStartY = newStartY;
		gapTarget = newTarget;
	}

	private void build(double min, double max) {
		top = -MARGIN;
		bottom = view.getHeight() + MARGIN;
		open = false;
		if (min > view.getXmin()) {
			endPoint(min, Math.min(Math.ceil(min / step) * step, max));
		}
		for (int i = 0; i < count; i++) {
			double x = (first + i) * step;
			if (x < min || x > max) {
				continue;
			}
			point(x, ys[i]);
			if (i + 1 < count && (first + i + 1) * step <= max) {
				gap(i);
			}
		}
		if (max < view.getXmax()) {
			endPoint(max, Math.max(Math.floor(max / step) * step, min));
		}
		flush();
	}

	// rounding can leave the function undefined right at the end of its domain, so the
	// end is moved towards the sample at inside until it is defined
	private void endPoint(double end, double insideStart) {
		double x = end;
		double y = evaluator.value(x);
		double inside = insideStart;
		double insideY = evaluator.value(inside);
		if (!Double.isFinite(y) && Double.isFinite(insideY)) {
			for (int k = 0; k < SPLIT_BISECTIONS; k++) {
				double mid = (x + inside) / 2;
				double value = evaluator.value(mid);
				if (Double.isFinite(value)) {
					inside = mid;
					insideY = value;
				} else {
					x = mid;
				}
			}
			x = inside;
			y = insideY;
		}
		point(x, y);
	}

	private void gap(int i) {
		double y0 = ys[i];
		double y1 = ys[i + 1];
		boolean defined0 = Double.isFinite(y0);
		boolean defined1 = Double.isFinite(y1);
		if (defined0 && defined1) {
			if (Math.abs(y1 - y0) * view.getYscale() < JUMP_PIXELS) {
				// a small sign change can still hide a pole when zoomed far out
				if (!findHiddenPoles || (y0 < 0) == (y1 < 0)) {
					return;
				}
				if (gaps[i] == UNKNOWN || gapTarget[i] != 0) {
					gaps[i] = findGap(i, 0);
				}
			} else {
				// only the visible part of the step matters, so the test aims at it
				double low = Math.max(Math.min(y0, y1), view.getYmin());
				double high = Math.min(Math.max(y0, y1), view.getYmax());
				if (low > high) {
					return;
				}
				if (gaps[i] == UNKNOWN || !(gapTarget[i] >= low && gapTarget[i] <= high)) {
					gaps[i] = findGap(i, (low + high) / 2);
				}
			}
		} else if (!defined0 && !defined1) {
			return;
		} else if (gaps[i] == UNKNOWN) {
			gaps[i] = findGap(i, Double.NaN);
		}
		if (gaps[i] == SPLIT) {
			point(splitEndX[i], splitEndY[i]);
			flush();
			open = false;
			point(splitStartX[i], splitStartY[i]);
		}
	}

	// narrows the gap down to where the function crosses the target, jumps or stops being defined
	private byte findGap(int i, double target) {
		double left = (first + i) * step;
		double right = left + step;
		double leftY = ys[i];
		double rightY = ys[i + 1];
		boolean bothDefined = Double.isFinite(leftY) && Double.isFinite(rightY);
		double initialJump = Math.abs(rightY - leftY);
		boolean leftBelow = leftY < target;
		gapTarget[i] = target;
		for (int k = 0; k < SPLIT_BISECTIONS; k++) {
			double mid = (left + right) / 2;
			double value = evaluator.value(mid);
			boolean leftSide;
			if (bothDefined) {
				if (!Double.isFinite(value)) {
					break;
				}
				leftSide = value < target == leftBelow;
			} else {
				leftSide = Double.isFinite(value) == Double.isFinite(leftY);
			}
			if (leftSide) {
				left = mid;
				leftY = value;
			} else {
				right = mid;
				rightY = value;
			}
		}
		// a continuous function closes in on the target value, a jump stays open
		if (bothDefined && Math.abs(rightY - leftY) <= initialJump * CONTINUITY_TOLERANCE) {
			return CONTINUOUS;
		}
		splitEndX[i] = left;
		splitEndY[i] = leftY;
		splitStartX[i] = right;
		splitStartY[i] = rightY;
		return SPLIT;
	}

	private void point(double x, double y) {
		if (!Double.isFinite(y)) {
			flush();
			open = false;
			return;
		}
		double sx = view.toScreenCoordXd(x);
		double sy = view.toScreenCoordYd(y);
		if (labelPoint == null && sy >= 0 && sy <= view.getHeight()) {
			labelPoint = new GPoint((int) sx, (int) sy);
		}
		sy = sy < top ? top : (sy > bottom ? bottom : sy);
		int col = (int) Math.floor(sx);
		if (open && col == column) {
			if (sy < minY) {
				minX = sx;
				minY = sy;
				minFirst = false;
			}
			if (sy > maxY) {
				maxX = sx;
				maxY = sy;
				minFirst = true;
			}
			endX = sx;
			endY = sy;
			return;
		}
		if (open) {
			flush();
			lineTo(sx, sy);
		} else {
			path.moveTo(sx, sy);
			lastX = sx;
			lastY = sy;
			open = true;
		}
		column = col;
		minX = maxX = endX = sx;
		minY = maxY = endY = sy;
	}

	private void flush() {
		if (!open) {
			return;
		}
		if (minFirst) {
			lineTo(minX, minY);
			lineTo(maxX, maxY);
		} else {
			lineTo(maxX, maxY);
			lineTo(minX, minY);
		}
		lineTo(endX, endY);
		minX = maxX = endX = lastX;
		minY = maxY = endY = lastY;
	}

	private void lineTo(double x, double y) {
		if (x != lastX || y != lastY) {
			path.lineTo(x, y);
			lastX = x;
			lastY = y;
		}
	}
}
