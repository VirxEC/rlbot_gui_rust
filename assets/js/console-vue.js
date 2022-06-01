const invoke = window.__TAURI__.invoke;
const listen = window.__TAURI__.event.listen;

export default {
	name: 'console',
	template: /*html*/`
	<div class="overflow-auto flex-grow-1">
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
	<b-container fluid>
		<b-card no-body class="bot-pool p-1">
			<span :class="text.err ? 'text-danger' : ''" v-for="text in consoleTexts.slice().reverse()">
				{{ text.text }}<br>
			</span>
		</b-card>
	</b-container>
	</div>
	`,
	components: {},
	data () {
		return {
			consoleTexts: [],
			newTextListener: listen('new-console-text', event => {
				event.payload.forEach(update => {
					if (update.replace_last) {
						this.consoleTexts.pop();
					}
					
					console.log(update);
					this.consoleTexts.push(update.content);
				});

				if (this.consoleTexts.length > 1200) {
					this.consoleTexts = this.consoleTexts.slice(0, this.consoleTexts.length - 1200);
				}
			})
		}
	},
	methods: {
		startup: function() {
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
