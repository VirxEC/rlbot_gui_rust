const invoke = window.__TAURI__.invoke;
const listen = window.__TAURI__.event.listen;

export default {
	name: 'console',
	template: /*html*/`
	<div class="overflow-hidden flex-grow-1">
	<b-navbar class="navbar">
		<b-navbar-brand>
			<img class="logo" src="imgs/rlbot_logo.png">
			<span class="rlbot-brand" style="flex: 1">RLBot Console</span>
		</b-navbar-brand>
		<b-navbar-nav class="ml-auto">
			<b-button @click="$router.back()" variant="dark" class="ml-2">
				Hide
			</b-button>
		</b-navbar-nav>
	</b-navbar>
	<b-container fluid>
		<b-card no-body class="bot-pool console-text-pool p-1">
			<b-form @submit="onSubmit" novalidate>
				<b-form-input v-on:keyup.down="onDown" v-on:keyup.up="onUp" v-model="inputCommand" id="console-input" placeholder="Enter command..."></b-form-input>
			</b-form>
			<hr>

			<DynamicScroller
				ref="scroller"
				:items="consoleTexts"
				:min-item-size="24"
				class="scroller"
			>
				<DynamicScrollerItem
					slot-scope="{ item, index, active }"
					:item="item"
					:active="active"
					:data-index="index"
				>
					<span :style="'color:' + (item.content.color ? item.content.color : 'black') + ';'">
						<span>{{ item.content.text }}<span>
					</span>
				</DynamicScrollerItem>
			</DynamicScroller>

		</b-card>
	</b-container>
	</div>
	`,
	components: {},
	data () {
		return {
			inputCommand: "",
			savedInputCommand: "",
			previousCommands: [],
			commandsIndex: -1,
			consoleTexts: [],
			texts: 0,
			newTextListener: listen('new-console-text', event => {
				let update = event.payload;
				if (update.replace_last) {
					this.consoleTexts.pop();
				}

				this.consoleTexts.push({ 'id': this.texts, 'content': update.content });
				this.texts++;
				this.$refs.scroller.scrollToBottom()

				if (this.consoleTexts.length > 800) {
					this.consoleTexts.shift();
				}
			})
		}
	},
	methods: {
		onUp: function(event) {
			if (this.commandsIndex < this.previousCommands.length - 1) {
				if (this.commandsIndex == -1) {
					this.savedInputCommand = this.inputCommand;
				}

				this.commandsIndex += 1;
				this.inputCommand = this.previousCommands[this.commandsIndex];
			}
		},
		onDown: function(event) {
			if (this.commandsIndex > -1) {
				this.commandsIndex -= 1;

				if (this.commandsIndex == -1) {
					this.inputCommand = this.savedInputCommand;
				} else {
					this.inputCommand = this.previousCommands[this.commandsIndex];
				}
			}
		},
		onSubmit: function() {
			if (this.inputCommand.length == 0) {
				return;
			}

			if (this.previousCommands[0] != this.inputCommand) {
				this.previousCommands.unshift(this.inputCommand)
			}

			invoke("run_command",  { input: this.inputCommand }).catch(error => console.error(error));

			this.inputCommand = "";
			this.savedInputCommand = "";
			this.commandsIndex = -1;
		},
		startup: function() {
			invoke("get_console_texts").then(texts => {
				texts.forEach(content => {
					this.consoleTexts.push({ 'id': this.texts, 'content': content });
					this.texts++;
				});

				this.$refs.scroller.scrollToBottom()
			});

			invoke("get_console_input_commands").then(commands => {
				this.previousCommands = commands;
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
