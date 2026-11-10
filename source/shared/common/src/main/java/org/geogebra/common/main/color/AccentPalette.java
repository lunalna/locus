package org.geogebra.common.main.color;

import java.util.HashMap;
import java.util.Map;

/**
 * Accent color shades. Resource packs override them at startup, before the color constants
 * that use them are initialized.
 */
public final class AccentPalette {

	/** Shades used by the color constants with their default colors as 0xRRGGBB. */
	public static final Map<Integer, Integer> DEFAULTS = Map.of(
			100, 0xFDF2F8, 600, 0xDB2777, 700, 0xBE185D);

	private static final Map<Integer, Integer> overrides = new HashMap<>();

	private AccentPalette() {
	}

	/**
	 * @param shade shade (100 to 900)
	 * @param rgb color as 0xRRGGBB
	 */
	public static void set(int shade, int rgb) {
		overrides.put(shade, rgb);
	}

	/**
	 * @param shade one of the shades in {@link #DEFAULTS}
	 * @return color as 0xRRGGBB
	 */
	public static int get(int shade) {
		return overrides.getOrDefault(shade, DEFAULTS.get(shade));
	}
}
