export const manifest = (() => {
function __memo(fn) {
	let value;
	return () => value ??= (value = fn());
}

return {
	appDir: "_app",
	appPath: "_app",
	assets: new Set(["favicon.png","themes/art-nouveau/theme.css"]),
	mimeTypes: {".png":"image/png",".css":"text/css"},
	_: {
		client: {start:"_app/immutable/entry/start.DIXC2w-U.js",app:"_app/immutable/entry/app.DSJcxGOV.js",imports:["_app/immutable/entry/start.DIXC2w-U.js","_app/immutable/chunks/Bf_MS-e1.js","_app/immutable/chunks/CCUsgSTd.js","_app/immutable/chunks/YOeQcqYW.js","_app/immutable/entry/app.DSJcxGOV.js","_app/immutable/chunks/CK4f36p2.js","_app/immutable/chunks/CCUsgSTd.js","_app/immutable/chunks/YOeQcqYW.js","_app/immutable/chunks/CJM1f-Ba.js","_app/immutable/chunks/CM2QGpeL.js"],stylesheets:[],fonts:[],uses_env_dynamic_public:false},
		nodes: [
			__memo(() => import('./nodes/0.js')),
			__memo(() => import('./nodes/1.js'))
		],
		remotes: {
			
		},
		routes: [
			
		],
		prerendered_routes: new Set(["/"]),
		matchers: async () => {
			
			return {  };
		},
		server_assets: {}
	}
}
})();
