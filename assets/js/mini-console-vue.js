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
						item.content.text,
					]"
					:data-index="index"
					:data-active="active"
					class="console-text-item"
				>
					<span :style="'color:' + (item.content.color ? item.content.color : 'black') + ';'">{{ item.content.text }}</span>
				</DynamicScrollerItem>
			</template>
		</DynamicScroller>
	</b-card>
	`,
	data () {
		return {
			consoleTexts: [],
			texts: 0,
			newTextListener: listen('new-console-text', event => {
				let update = event.payload;
				if (update.replace_last) {
					this.consoleTexts.pop();
				}

				this.consoleTexts.push({ 'id': this.texts, 'content': update.content });
				this.texts++;
				
				if (this.consoleTexts.length > 420) {
					this.consoleTexts.shift();
				}

				this.$refs.scroller.scrollToBottom();
			})
		}
	},
}
