/*
 * GeoGebra - Dynamic Mathematics for Everyone
 * Copyright (c) GeoGebra GmbH, Altenbergerstr. 69, 4040 Linz, Austria
 * https://www.geogebra.org
 *
 * This file is licensed by GeoGebra GmbH under the EUPL 1.2 licence and
 * may be used under the EUPL 1.2 in compatible projects (see Article 5
 * and the Appendix of EUPL 1.2 for details).
 * You may obtain a copy of the licence at:
 * https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12
 *
 * Note: The overall GeoGebra software package is free to use for
 * non-commercial purposes only.
 * See https://www.geogebra.org/license for full licensing details
 */


package org.geogebra.common.euclidian.plot;

import org.geogebra.common.kernel.arithmetic.Evaluatable;
import org.geogebra.common.kernel.geos.GeoCurveCartesian;
import org.geogebra.common.kernel.geos.GeoElement;
import org.geogebra.common.kernel.geos.GeoFunction;
import org.geogebra.common.kernel.kernelND.CurveEvaluable;

/**
 * A curve whose points come from a compiled evaluator instead of the expression interpreter.
 * The evaluator has the values of all other objects built in, so it has to be created again
 * after the curve was updated.
 */
public final class CompiledCurve implements CurveEvaluable {

	/** Compiled curve coordinates. */
	public interface Evaluator {
		/**
		 * @param t curve parameter
		 * @param out point of the curve
		 */
		void evaluate(double t, double[] out);
	}

	/** Turns a curve into a faster evaluator, or null; set by platforms that can compile. */
	public static java.util.function.Function<GeoCurveCartesian, Evaluator> compiler =
			curve -> null;

	private final CurveEvaluable curve;
	private final Evaluator evaluator;

	private CompiledCurve(CurveEvaluable curve, Evaluator evaluator) {
		this.curve = curve;
		this.evaluator = evaluator;
	}

	/**
	 * @param curve curve or function that is not swapped
	 * @return compiled equivalent of the curve, or the curve itself if it cannot be compiled
	 */
	public static CurveEvaluable of(CurveEvaluable curve) {
		Evaluator evaluator = null;
		if (curve instanceof GeoFunction) {
			Evaluatable function = FunctionPlotter.compiler.apply((GeoFunction) curve);
			if (function != null) {
				evaluator = (t, out) -> {
					out[0] = t;
					out[1] = function.value(t);
				};
			}
		} else if (curve.getClass() == GeoCurveCartesian.class) {
			evaluator = compiler.apply((GeoCurveCartesian) curve);
		}
		return evaluator == null ? curve : new CompiledCurve(curve, evaluator);
	}

	@Override
	public void evaluateCurve(double t, double[] out) {
		evaluator.evaluate(t, out);
	}

	@Override
	public double getMinParameter() {
		return curve.getMinParameter();
	}

	@Override
	public double getMaxParameter() {
		return curve.getMaxParameter();
	}

	@Override
	public double[] newDoubleArray() {
		return curve.newDoubleArray();
	}

	@Override
	public double distanceMax(double[] p1, double[] p2) {
		return curve.distanceMax(p1, p2);
	}

	@Override
	public double[] getDefinedInterval(double a, double b) {
		return curve.getDefinedInterval(a, b);
	}

	@Override
	public boolean getTrace() {
		return curve.getTrace();
	}

	@Override
	public boolean isClosedPath() {
		return curve.isClosedPath();
	}

	@Override
	public boolean isFunctionInX() {
		return curve.isFunctionInX();
	}

	@Override
	public GeoElement toGeoElement() {
		return curve.toGeoElement();
	}

	@Override
	public double getMinDistX() {
		return curve.getMinDistX();
	}
}
