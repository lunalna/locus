package org.geogebra.web.full.gui.view.assistant;

import elemental2.dom.DomGlobal;
import jsinterop.annotations.JsFunction;
import jsinterop.base.Js;

/** Runs a request typed into the Algebra input after "?" with the host page's AI. */
public final class PromptBridge {

	/** Called once the request is done, with an error or the AI's reply, either may be null. */
	@JsFunction
	public interface Done {
		/**
		 * @param error error message
		 * @param reply the AI's answer
		 */
		void finish(String error, String reply);
	}

	@JsFunction
	private interface Prompt {
		void run(String text, Done done);
	}

	private PromptBridge() {
	}

	/**
	 * @return whether the host page allows natural language input
	 */
	public static boolean isEnabled() {
		return Js.isTruthy(Js.asPropertyMap(DomGlobal.window).get("locusPromptEnabled"));
	}

	/**
	 * @param text request in plain language
	 * @param done called when the AI is done
	 */
	public static void run(String text, Done done) {
		Js.<Prompt>uncheckedCast(Js.asPropertyMap(DomGlobal.window).get("locusPrompt")).run(text, done);
	}
}
