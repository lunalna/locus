package org.geogebra.web.resources;

import java.util.ArrayList;
import java.util.List;

import org.geogebra.common.awt.ThemeColors;

import elemental2.dom.CSSStyleDeclaration;
import elemental2.dom.DomGlobal;
import elemental2.dom.Element;
import elemental2.dom.StyleSheet;
import elemental2.dom.StyleSheetList;
import jsinterop.base.Js;
import jsinterop.base.JsPropertyMap;

/**
 * Maps the colors of the page's stylesheets with the active theme.
 */
public final class ThemeStyles {

	// original values of the mapped declarations, so another theme can map them again
	private static final List<Declaration> declarations = new ArrayList<>();

	private static final class Declaration {
		private final CSSStyleDeclaration style;
		private final String name;
		private final String value;
		private final String priority;

		private Declaration(CSSStyleDeclaration style, String name) {
			this.style = style;
			this.name = name;
			this.value = style.getPropertyValue(name);
			this.priority = style.getPropertyPriority(name);
		}

		private void map() {
			String mapped = ThemeColors.css(value);
			if (!mapped.equals(style.getPropertyValue(name))) {
				style.setProperty(name, mapped, priority);
			}
		}
	}

	private ThemeStyles() {
	}

	/**
	 * Maps all stylesheets again after the theme changed.
	 */
	public static void update() {
		declarations.forEach(Declaration::map);
		apply();
	}

	/**
	 * Maps all stylesheets that are loaded and not mapped yet.
	 */
	public static void apply() {
		if (!ThemeColors.isActive()) {
			return;
		}
		StyleSheetList sheets = Js.uncheckedCast(
				Js.asPropertyMap(DomGlobal.document).get("styleSheets"));
		for (int i = 0; i < sheets.length; i++) {
			StyleSheet sheet = sheets.item(i);
			JsPropertyMap<Object> props = Js.asPropertyMap(sheet);
			if (props.has("locusThemed") || isThemeSheet(sheet)) {
				continue;
			}
			try {
				mapRules(props.get("cssRules"));
				props.set("locusThemed", true);
			} catch (Throwable expected) {
				// still loading or from another origin
			}
		}
	}

	// the pack and theme stylesheets already use the theme's colors
	private static boolean isThemeSheet(StyleSheet sheet) {
		Element owner = Js.uncheckedCast(sheet.ownerNode);
		String id = owner == null ? "" : owner.id;
		return id.startsWith("ggbstyle_pack_theme") || id.startsWith("locus_theme");
	}

	private static void mapRules(Object rules) {
		JsPropertyMap<Object> list = Js.asPropertyMap(rules);
		int length = Js.asInt(list.get("length"));
		for (int i = 0; i < length; i++) {
			JsPropertyMap<Object> rule = Js.asPropertyMap(list.get(String.valueOf(i)));
			if (rule.has("style")) {
				mapStyle(Js.uncheckedCast(rule.get("style")));
			}
			if (rule.get("cssRules") != null) {
				mapRules(rule.get("cssRules"));
			}
		}
	}

	// values that ThemeColors.mapText may change
	private static boolean hasColor(String value) {
		return value.contains("#") || value.contains("rgb") || value.contains("white")
				|| value.contains("black");
	}

	private static void mapStyle(CSSStyleDeclaration style) {
		for (int i = 0; i < style.length; i++) {
			String name = style.item(i);
			String value = style.getPropertyValue(name);
			// accent variables come from the theme, shadows stay dark, data URLs must stay intact
			if (name.contains("shadow") || value.contains("var(") || value.contains("url(")
					|| !hasColor(value)) {
				continue;
			}
			Declaration declaration = new Declaration(style, name);
			declarations.add(declaration);
			declaration.map();
		}
	}
}
