const listen = window.__TAURI__.event.listen;

export default {
	name: 'mini-console',
	template: /*html*/`
	<b-card no-body class="console-text-pool p-1" style="max-height: 80vh">
		<span :style="'color:' + (text.color ? text.color : 'black') + ';'" v-for="text in consoleTexts">
			<span>{{ text.text }}</span><br>
		</span>
	</b-card>
	`,
	data () {
		return {
			consoleTexts: [],
			newTextListener: listen('new-console-text', event => {
				let update = event.payload;
				if (update.replace_last) {
					this.consoleTexts.pop();
				}

				this.consoleTexts.unshift(update.content);

				if (this.consoleTexts.length > 240) {
					this.consoleTexts.splice(240, this.consoleTexts.length - 240);
				}
			})
		}
	},
}
