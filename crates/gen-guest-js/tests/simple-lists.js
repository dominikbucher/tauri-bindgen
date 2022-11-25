const invoke = window.__TAURI_INVOKE__;
/**
 * @param {Uint32Array} l
 */
export async function simpleList1(l) {
	await invoke("plugin:d40a3203ef48115d|simple_list1", { l: l });
}
/**
 * @returns {Promise<Uint32Array>}
 */
export async function simpleList2() {
	const result = await invoke("plugin:d40a3203ef48115d|simple_list2");
	return result;
}
/**
 * @param {Uint32Array[]} l
 * @returns {Promise<Uint32Array[]>}
 */
export async function simpleList4(l) {
	const result = await invoke("plugin:d40a3203ef48115d|simple_list4", { l: l });
	return result;
}

