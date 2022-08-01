const listen = window.__TAURI__.event.listen;

export default {
	name: 'mini-console',
	template: /*html*/`
	<b-card no-body class="console-text-pool p-1" style="max-height: 80vh">
		<RecycleScroller
		ref="scroller"
		class="scroller"
		:items="consoleTexts"
		:item-size="26"
		v-slot="{ item }"
		>
			<span class="console-text-item" :style="'color:' + (item.content.color ? item.content.color : 'black') + ';'">
				<span>{{ item.content.text }}</span>
			</span>
		</RecycleScroller>
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

				this.consoleTexts.push({ 'id': this.texts, 'content': update.content });
				this.$refs.scroller.scrollToItem(this.texts)
				this.texts++;

				if (this.consoleTexts.length > 800) {
					this.consoleTexts.shift();
				}
			})
		}
	},
}
