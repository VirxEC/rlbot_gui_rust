const invoke = window.__TAURI__.invoke;

export default {
	name: 'console',
	template: /*html*/`
	<div class="noscroll-flex flex-grow-1">
	<b-navbar class="navbar">
		<b-navbar-brand>
			<img class="logo" src="imgs/rlbot_logo.png">
			<span class="rlbot-brand" style="flex: 1">RLBot Console</span>
		</b-navbar-brand>
		<b-navbar-nav class="ml-auto">
			<b-button @click="$router.replace('/')" variant="dark" class="ml-2">
				Hide
			</b-button>
		</b-navbar-nav>
	</b-navbar>
	<b-container fluid class="rlbot-main-config noscroll-flex flex-grow-1">
		<b-card no-body>
			<div class="center-flex my-2">
				<p v-for="text in consoleTexts">{{ text }}</p>
			</div>
		</b-card>
	</b-container>
	</div>
	`,
	components: {},
	data () {
		return {
			consoleTexts: []
		}
	},
	methods: {
		startup: function() {
			this.consoleTexts.push("Welcome to the RLBot Console!");
		}
	},
	computed: {},
	created: function () {
		this.startup()
	},
	watch: {
		// call again the method if the route changes
		'$route': 'startup'
	},
};
