package org.geogebra.web.full.main;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.function.IntConsumer;
import java.util.function.IntSupplier;
import java.util.stream.Collectors;
import java.util.stream.IntStream;

import org.geogebra.common.euclidian.EuclidianView;
import org.geogebra.common.kernel.ConstructionDefaults;
import org.geogebra.common.kernel.geos.PointProperties;
import org.geogebra.common.main.App;
import org.geogebra.common.main.Localization;
import org.geogebra.common.main.settings.LabelVisibility;
import org.geogebra.common.ownership.GlobalScope;
import org.geogebra.common.properties.ActionableProperty;
import org.geogebra.common.properties.EnumeratedProperty;
import org.geogebra.common.properties.IconsEnumeratedProperty;
import org.geogebra.common.properties.NamedEnumeratedProperty;
import org.geogebra.common.properties.Property;
import org.geogebra.common.properties.PropertyCollection;
import org.geogebra.common.properties.PropertyCollectionWithLead;
import org.geogebra.common.properties.RangeProperty;
import org.geogebra.common.properties.ValuedProperty;
import org.geogebra.common.properties.aliases.BooleanProperty;
import org.geogebra.common.properties.aliases.ColorProperty;
import org.geogebra.common.properties.aliases.StringProperty;
import org.geogebra.common.properties.factory.PropertiesArray;
import org.geogebra.common.properties.impl.AbstractNamedEnumeratedProperty;
import org.geogebra.common.properties.impl.algebra.SortByProperty;
import org.geogebra.common.properties.impl.general.LabelingProperty;
import org.geogebra.common.properties.impl.graphics.AxesVisibilityProperty;
import org.geogebra.common.properties.impl.graphics.GridVisibilityProperty;
import org.geogebra.common.properties.impl.graphics.PointCapturingProperty;
import org.geogebra.common.util.StringUtil;
import org.geogebra.gwtutil.JsRunnable;
import org.geogebra.web.html5.gui.util.BrowserStorage;

import elemental2.core.Global;
import elemental2.core.JsArray;
import elemental2.dom.DomGlobal;
import jsinterop.annotations.JsFunction;
import jsinterop.base.Js;
import jsinterop.base.JsPropertyMap;

/**
 * Exposes the app's settings to the Locus settings modal as window.locusAppSettings.
 * The general and algebra tabs come as one flat list, the other tabs (graphics, spreadsheet, CAS)
 * as trees. Every entry has a key that finds its property again when it is changed.
 */
public final class LocusAppSettings {

	private static final List<String> FLAT_TABS = List.of("General", "Algebra");
	private static final String SHARED_KEY = "locus.sharedSettings";

	@JsFunction
	private interface Getter {
		Object get();
	}

	@JsFunction
	private interface Setter {
		/** @return an error message for invalid text, or null */
		String set(String key, Object value);
	}

	/** A size from 1 to max, used for the default line thickness and point size. */
	private static final class SizeProperty extends AbstractNamedEnumeratedProperty<Integer> {
		private final IntSupplier getter;
		private final IntConsumer setter;

		SizeProperty(Localization loc, String name, int max, IntSupplier getter, IntConsumer setter) {
			super(loc, name);
			this.getter = getter;
			this.setter = setter;
			setNamedValues(IntStream.rangeClosed(1, max).boxed()
					.map(size -> Map.entry(size, String.valueOf(size))).collect(Collectors.toList()));
		}

		@Override
		public Integer getValue() {
			return getter.getAsInt();
		}

		@Override
		protected void doSetValue(Integer value) {
			setter.accept(value);
		}
	}

	private LocusAppSettings() {
	}

	static void register(AppWFull app) {
		JsPropertyMap<Object> bridge = JsPropertyMap.of();
		bridge.set("list", (Getter) () -> list(app));
		bridge.set("tabs", (Getter) () -> tabsTree(app));
		bridge.set("set", (Setter) (key, value) -> set(app, key, value));
		bridge.set("reset", (JsRunnable) () -> {
			app.restoreSettings();
			saveShared(app);
		});
		bridge.set("exam", (Getter) () -> GlobalScope.isExamActive(app));
		Js.asPropertyMap(DomGlobal.window).set("locusAppSettings", bridge);
	}

	/** Applies the settings of the flat list that were last changed in any app. */
	static void applyShared(App app) {
		JsPropertyMap<String> shared = readShared();
		for (Property property : flatProperties(app)) {
			String value = shared.get(property.getRawName());
			if (value == null || value.equals(valueOf(property))) {
				continue;
			}
			if (property instanceof BooleanProperty toggle) {
				toggle.setValue(Boolean.parseBoolean(value));
			} else {
				EnumeratedProperty<?> choice = (EnumeratedProperty<?>) property;
				List<String> values = choice.getValues().stream().map(String::valueOf).toList();
				if (values.contains(value)) {
					choice.setIndex(values.indexOf(value));
				}
			}
		}
	}

	private static void saveShared(App app) {
		JsPropertyMap<String> shared = readShared();
		for (Property property : flatProperties(app)) {
			shared.set(property.getRawName(), valueOf(property));
		}
		BrowserStorage.LOCAL.setItem(SHARED_KEY, Global.JSON.stringify(shared));
	}

	private static JsPropertyMap<String> readShared() {
		String json = BrowserStorage.LOCAL.getItem(SHARED_KEY);
		return json == null ? JsPropertyMap.of() : Js.cast(Global.JSON.parse(json));
	}

	private static String valueOf(Property property) {
		return String.valueOf(((ValuedProperty<?>) property).getValue());
	}

	private static List<PropertiesArray> tabs(App app) {
		return app.getConfig().createPropertiesFactory()
				.createProperties(app, app.getLocalization(), app.appScope.propertiesRegistry)
				.stream().filter(Objects::nonNull).toList();
	}

	private static List<Property> flatProperties(App app) {
		List<Property> properties = new ArrayList<>();
		boolean algebra = false;
		for (PropertiesArray tab : tabs(app)) {
			if (FLAT_TABS.contains(tab.getRawName())) {
				algebra |= "Algebra".equals(tab.getRawName());
				properties.addAll(List.of(tab.getProperties()));
			}
		}
		// apps without an algebra view (Scientific, Probability, Whiteboard) have no objects to style
		if (algebra) {
			for (Property extra : extraProperties(app)) {
				if (properties.stream().noneMatch(p -> p.getRawName().equals(extra.getRawName()))) {
					properties.add(extra);
				}
			}
		}
		properties.removeIf(property -> !property.isAvailable()
				|| !(property instanceof NamedEnumeratedProperty || property instanceof BooleanProperty));
		return properties;
	}

	private static List<Property> extraProperties(App app) {
		Localization loc = app.getLocalization();
		EuclidianView view = app.getEuclidianView1();
		ConstructionDefaults defaults = app.getKernel().getConstruction().getConstructionDefaults();
		LabelingProperty labeling = app.isUnbundledOrWhiteboard()
				? new LabelingProperty(loc, app.getSettings().getLabelSettings())
				: new LabelingProperty(loc, app.getSettings().getLabelSettings(),
						LabelVisibility.Automatic, LabelVisibility.AlwaysOn,
						LabelVisibility.AlwaysOff, LabelVisibility.PointsOnly);
		return List.of(
				new SortByProperty(app.getSettings().getAlgebra(), loc),
				labeling,
				new PointCapturingProperty(loc, view),
				new GridVisibilityProperty(loc, view.getSettings()),
				new AxesVisibilityProperty(loc, view.getSettings()),
				// read from the default objects, since those are what the saved preferences restore
				new SizeProperty(loc, "Thickness", 13,
						() -> defaults.getDefaultGeo(ConstructionDefaults.DEFAULT_SEGMENT)
								.getLineThickness(),
						defaults::setDefaultLineThickness),
				new SizeProperty(loc, "PointSize", 9,
						() -> ((PointProperties) defaults.getDefaultGeo(
								ConstructionDefaults.DEFAULT_POINT_FREE)).getPointSize(),
						size -> defaults.setDefaultPointSize(size, size)));
	}

	private static JsArray<Object> list(App app) {
		JsArray<Object> list = new JsArray<>();
		List<Property> properties = flatProperties(app);
		for (int i = 0; i < properties.size(); i++) {
			list.push(entry(properties.get(i), "f." + i, app.getLocalization()));
		}
		return list;
	}

	private static JsArray<Object> tabsTree(App app) {
		JsArray<Object> list = new JsArray<>();
		List<PropertiesArray> tabs = tabs(app);
		for (int i = 0; i < tabs.size(); i++) {
			PropertiesArray tab = tabs.get(i);
			if (!FLAT_TABS.contains(tab.getRawName())) {
				JsPropertyMap<Object> entry = JsPropertyMap.of();
				entry.set("id", tab.getRawName());
				entry.set("name", tab.getName());
				entry.set("items", items(tab.getProperties(), "t." + i, app.getLocalization()));
				list.push(entry);
			}
		}
		return list;
	}

	private static JsArray<Object> items(Property[] properties, String key, Localization loc) {
		JsArray<Object> items = new JsArray<>();
		for (int i = 0; i < properties.length; i++) {
			JsPropertyMap<Object> entry = entry(properties[i], key + "." + i, loc);
			if (entry != null) {
				items.push(entry);
			}
		}
		return items;
	}

	private static JsPropertyMap<Object> entry(Property property, String key, Localization loc) {
		if (!property.isAvailable()) {
			return null;
		}
		JsPropertyMap<Object> entry = JsPropertyMap.of();
		entry.set("key", key);
		entry.set("id", property.getRawName());
		entry.set("name", property.getName());
		entry.set("enabled", property.isEnabled());
		// a Double is a JS number in GWT, an Integer is not
		if (property instanceof BooleanProperty toggle) {
			entry.set("type", "toggle");
			entry.set("value", toggle.getValue());
		} else if (property instanceof NamedEnumeratedProperty<?> choice) {
			entry.set("type", "choice");
			entry.set("values", new JsArray<>(choice.getValueNames()));
			entry.set("value", (double) choice.getIndex());
		} else if (property instanceof IconsEnumeratedProperty<?> icons) {
			entry.set("type", "choice");
			entry.set("values", new JsArray<>(iconNames(icons, loc)));
			entry.set("value", (double) icons.getIndex());
		} else if (property instanceof ColorProperty color) {
			entry.set("type", "color");
			entry.set("values", new JsArray<>(color.getValues().stream()
					.map(c -> c == null ? "" : "#" + StringUtil.toHexString(c)).toArray(String[]::new)));
			entry.set("value", (double) color.getIndex());
		} else if (property instanceof RangeProperty<?> range) {
			entry.set("type", "range");
			entry.set("min", range.getMin().doubleValue());
			entry.set("max", range.getMax().doubleValue());
			entry.set("step", range.getStep().doubleValue());
			entry.set("value", range.getValue().doubleValue());
		} else if (property instanceof StringProperty text) {
			entry.set("type", "text");
			entry.set("value", text.getValue());
		} else if (property instanceof ActionableProperty) {
			entry.set("type", "action");
		} else if (property instanceof PropertyCollection<?> collection) {
			entry.set("type", "group");
			if (property instanceof PropertyCollectionWithLead withLead) {
				entry.set("lead", entry(withLead.leadProperty, key + ".lead", loc));
			}
			entry.set("items", items(collection.getProperties(), key, loc));
		} else {
			return null;
		}
		return entry;
	}

	// The icons have no names, so they come from the translated tooltips or the icon names.
	private static String[] iconNames(IconsEnumeratedProperty<?> icons, Localization loc) {
		String[] labels = icons.getToolTipLabels();
		if (labels != null) {
			return Arrays.stream(labels).map(loc::getMenu).toArray(String[]::new);
		}
		List<String[]> words = Arrays.stream(icons.getValueIcons())
				.map(resource -> resource.name().split("_")).toList();
		int common = 1;
		while (words.size() > 1 && sharesWord(words, common)) {
			common++;
		}
		int skip = common;
		return words.stream().map(w -> {
			String name = String.join(" ", Arrays.asList(w).subList(skip, w.length)).toLowerCase();
			return name.substring(0, 1).toUpperCase() + name.substring(1);
		}).toArray(String[]::new);
	}

	private static boolean sharesWord(List<String[]> words, int index) {
		return words.stream().allMatch(w -> w.length > index + 1 && w[index].equals(words.get(0)[index]));
	}

	private static Property find(App app, String key) {
		String[] parts = key.split("\\.");
		if ("f".equals(parts[0])) {
			return flatProperties(app).get(Integer.parseInt(parts[1]));
		}
		Property property = null;
		Property[] level = tabs(app).get(Integer.parseInt(parts[1])).getProperties();
		for (int i = 2; i < parts.length; i++) {
			property = "lead".equals(parts[i])
					? ((PropertyCollectionWithLead) property).leadProperty
					: level[Integer.parseInt(parts[i])];
			if (property instanceof PropertyCollection<?> collection) {
				level = collection.getProperties();
			}
		}
		return property;
	}

	@SuppressWarnings("unchecked")
	private static String set(AppWFull app, String key, Object value) {
		Property property = find(app, key);
		if (property instanceof BooleanProperty toggle) {
			toggle.setValue(Js.isTruthy(value));
		} else if (property instanceof EnumeratedProperty<?> choice) {
			choice.setIndex(Js.asInt(value));
		} else if (property instanceof RangeProperty<?> range) {
			if (range.getMin() instanceof Integer) {
				((RangeProperty<Integer>) range).setValue(Js.asInt(value));
			} else {
				((RangeProperty<Double>) range).setValue(Js.asDouble(value));
			}
		} else if (property instanceof StringProperty text) {
			String error = text.validateValue(Js.asString(value));
			if (error != null) {
				return error;
			}
			text.setValue(Js.asString(value));
		} else if (property instanceof ActionableProperty action) {
			action.performAction();
		}
		if (key.startsWith("f.")) {
			saveShared(app);
		}
		app.saveSettings();
		return null;
	}
}
