const listen = window.__TAURI__.event.listen;

export default {
	name: 'mini-console',
	template: /*html*/`
	<b-card no-body class="p-1" style="white-space: pre-wrap;">
		<span :style="'color:' + (text.color ? text.color : 'black') + ';'" v-for="text in consoleTexts.slice().reverse()">
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
				
				this.consoleTexts.push(update.content);

				if (this.consoleTexts.length > 1200) {
					this.consoleTexts = this.consoleTexts.slice(this.consoleTexts.length - 1200);
				}
			})
		}
	},
}
