const listen = window.__TAURI__.event.listen;

export default {
	name: 'mini-console',
	template: /*html*/`
	<b-card no-body class="console-text-pool p-1" style="max-height: 80vh">
		<DynamicScroller
			ref="scroller"
			:items="consoleTexts"
			:min-item-size="26"
			class="scroller"
		>
			<template v-slot="{ item, index, active }">
				<DynamicScrollerItem
					:item="item"
					:active="active"
					:size-dependencies="[
						item.content,
					]"
					:data-index="index"
					:data-active="active"
					class="console-text-item"
					v-html="item.content"
				></DynamicScrollerItem>
			</template>
		</DynamicScroller>
	</b-card>
	`,
	data () {
		return {
			consoleTexts: [],
			texts: 0,
			newTextListener: listen('new-console-texts', event => {
				event.payload.forEach(update => {
					if (update.replace_last) {
						this.consoleTexts.pop();
					}

					this.consoleTexts.push({ 'id': this.texts, 'content': update.content });
					this.texts++;
					
					if (this.consoleTexts.length > 420) {
						this.consoleTexts.shift();
					}
				})

				try {
					this.$refs.scroller.scrollToBottom();
				} catch (e) {} // ignore the error, it randomly happens sometimes but it still works
			})
		}
	},
}
