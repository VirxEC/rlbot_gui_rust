const invoke = window.__TAURI__.invoke;
const listen = window.__TAURI__.event.listen;

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
	<b-container fluid class="noscroll-flex flex-grow-1">
		<b-card no-body class="bot-pool noscroll-flex flex-grow-1">
			<div class="my-2">
				<div class="p-1 noscroll-flex">
					<div class="overflow-auto">
						<div v-for="text in consoleTexts" style="display: inline;">
							<p>{{ text }}</p>
						</div>
					</div>
				</div>
			</div>
		</b-card>
	</b-container>
	</div>
	`,
	components: {},
	data () {
		return {
			consoleTexts: [],
			newTextListener: listen('new-console-text', event => {
				console.log(event.payload);
				this.consoleTexts.push(...event.payload);
			})
		}
	},
	methods: {
		startup: function() {
			// this.consoleTexts.push("Welcome to the RLBot Console!");
			invoke("get_console_texts").then((texts) => {
				this.consoleTexts = texts;
			});
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
