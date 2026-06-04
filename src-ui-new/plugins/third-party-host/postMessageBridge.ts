// postMessage bridge for third-party plugin iframes.
// The iframe communicates with the parent window via postMessage.

export interface PluginHostMessage {
  type: 'data-update' | 'action-trigger' | 'resize';
  data?: unknown;
  actions?: unknown[];
  actionId?: string;
  width?: number;
  height?: number;
}

export interface HostToPluginMessage {
  pluginId: string;
  payload: unknown;
}

const ORIGIN_FILTER = '*'; // iframes from zlplugin:// have null origin

export function sendToIframe(
  iframe: HTMLIFrameElement,
  message: PluginHostMessage,
): void {
  iframe.contentWindow?.postMessage(message, ORIGIN_FILTER);
}

export function isValidPluginMessage(e: MessageEvent, pluginId: string): boolean {
  return (
    e.data?.pluginId === pluginId ||
    e.data?.payload !== undefined
  );
}
