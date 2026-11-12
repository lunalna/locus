package org.geogebra.web.full.gui;

import java.util.ArrayList;
import java.util.List;

import org.geogebra.web.full.gui.menu.icons.DefaultMenuIconResources;
import org.geogebra.web.html5.gui.view.IconSpec;
import org.geogebra.web.html5.gui.view.ImageIconSpec;
import org.geogebra.web.html5.gui.view.button.StandardButton;
import org.geogebra.web.html5.main.toolbox.CustomIconSpec;
import org.gwtproject.dom.client.Element;

import elemental2.core.JsArray;
import elemental2.dom.DomGlobal;
import jsinterop.annotations.JsFunction;
import jsinterop.annotations.JsPackage;
import jsinterop.annotations.JsType;
import jsinterop.base.Js;

/** Plugin commands for the toolbar, the main menu and the object context menu, from the host page. */
public final class LocusPluginEntries {

	/** A plugin command. */
	@JsType(isNative = true, namespace = JsPackage.GLOBAL, name = "Object")
	public static class Entry {
		public String plugin;
		public String id;
		public String title;
		public String icon;
	}

	@JsFunction
	private interface Entries {
		JsArray<Entry> get(String place, JsArray<String> types);
	}

	@JsFunction
	private interface Run {
		void run(String plugin, String id, JsArray<String> objects);
	}

	private LocusPluginEntries() {
	}

	/**
	 * @param place toolbar or menu
	 * @return the commands plugins add there
	 */
	public static List<Entry> get(String place) {
		return get(place, null);
	}

	/**
	 * @param place toolbar, menu or object
	 * @param types types of the selected objects, or null
	 * @return the commands plugins add there
	 */
	public static List<Entry> get(String place, List<String> types) {
		List<Entry> list = new ArrayList<>();
		Object entries = Js.asPropertyMap(DomGlobal.window).get("locusPluginEntries");
		if (entries != null) {
			JsArray<Entry> array = Js.<Entries>uncheckedCast(entries).get(place, toArray(types));
			for (int i = 0; i < array.length; i++) {
				list.add(array.getAt(i));
			}
		}
		return list;
	}

	/**
	 * @param entry command
	 * @return the plugin's icon, or the plugins icon
	 */
	public static IconSpec icon(Entry entry) {
		return entry.icon != null ? new CustomIconSpec(entry.icon)
				: new ImageIconSpec(DefaultMenuIconResources.INSTANCE.plugins());
	}

	/**
	 * @param entry command
	 * @param size icon size
	 * @param label whether the title shows below the icon
	 * @return a toolbar button that runs the command
	 */
	public static StandardButton button(Entry entry, int size, boolean label) {
		StandardButton button = new StandardButton(icon(entry), label ? entry.title : null, size, size);
		button.setTitle(entry.title);
		Element img = button.getElement().getElementsByTagName("img").getItem(0);
		if (img != null) {
			img.setAttribute("width", String.valueOf(size));
			img.setAttribute("height", String.valueOf(size));
		}
		button.addFastClickHandler(source -> run(entry, null));
		return button;
	}

	/**
	 * @param entry command
	 * @param objects names of the selected objects, or null
	 */
	public static void run(Entry entry, List<String> objects) {
		Js.<Run>uncheckedCast(Js.asPropertyMap(DomGlobal.window).get("locusPluginRun"))
				.run(entry.plugin, entry.id, toArray(objects));
	}

	/**
	 * Runs a command from the RunPlugin command in a construction's script.
	 * @param plugin plugin id
	 * @param id command id
	 * @param objects names of the objects it gets
	 */
	public static void script(String plugin, String id, List<String> objects) {
		Object script = Js.asPropertyMap(DomGlobal.window).get("locusPluginScript");
		if (script != null) {
			Js.<Run>uncheckedCast(script).run(plugin, id, toArray(objects));
		}
	}

	private static JsArray<String> toArray(List<String> list) {
		if (list == null) {
			return null;
		}
		JsArray<String> array = new JsArray<>();
		for (String item : list) {
			array.push(item);
		}
		return array;
	}
}
