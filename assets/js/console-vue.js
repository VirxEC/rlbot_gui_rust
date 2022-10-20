const invoke = window.__TAURI__.invoke;
const listen = window.__TAURI__.event.listen;
const MAX_LINES = 840;

export default {
  name: "console",
  template: /* html */ `
  <div class="overflow-hidden flex-grow-1">
  <b-navbar class="navbar">
    <b-navbar-brand>
      <img class="logo" src="imgs/rlbot_logo.png">
      <span class="rlbot-brand" style="flex: 1">RLBot Console</span>
    </b-navbar-brand>
    <b-navbar-nav class="ml-auto">
      <b-button @click="toggleScrollLock()" variant="dark" class="ml-2">
        <span v-if="scrollLock">Unlock scolling</span>
        <span v-else>Lock scolling</span>
      </b-button>
      <b-button @click="$router.back()" variant="dark" class="ml-2">
        Hide
      </b-button>
    </b-navbar-nav>
  </b-navbar>
  <b-container fluid>
    <b-card no-body class="bot-pool console-text-pool p-1">
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

      <hr>

      <b-form @submit="onSubmit" novalidate>
        <b-form-input v-on:keyup.down="onDown" v-on:keyup.up="onUp" v-model="inputCommand" id="console-input" placeholder="Enter command..."></b-form-input>
      </b-form>
    </b-card>
  </b-container>
  </div>
  `,
  components: {},
  data() {
    return {
      inputCommand: "",
      savedInputCommand: "",
      previousCommands: [],
      commandsIndex: -1,
      consoleTexts: [],
      texts: 0,
      userChoseLock: false,
      scrollLock: true,
      newTextListener: listen("new-console-texts", (event) => {
        event.payload.forEach((update) => {
          if (update.replace_last) {
            this.consoleTexts.pop();
          }

          this.consoleTexts.push({ id: this.texts, content: update.content });
          this.texts++;

          if (this.consoleTexts.length > MAX_LINES) {
            this.consoleTexts.shift();
          }
        });

        if (this.scrollLock) {
          if (this.consoleTexts.length >= MAX_LINES && !this.userChoseLock) {
            this.scrollLock = false;
          }

          try {
            this.$refs.scroller.scrollToBottom();
            // eslint-disable-next-line no-empty
          } catch (e) {} // ignore the error, it randomly happens sometimes but it still works
        }
      }),
    };
  },
  methods: {
    toggleScrollLock: function () {
      this.userChoseLock = true;
      this.scrollLock = !this.scrollLock;
      if (this.scrollLock) {
        this.$refs.scroller.scrollToBottom();
      }
    },
    onUp: function () {
      if (this.commandsIndex < this.previousCommands.length - 1) {
        if (this.commandsIndex === -1) {
          this.savedInputCommand = this.inputCommand;
        }

        this.commandsIndex += 1;
        this.inputCommand = this.previousCommands[this.commandsIndex];
      }
    },
    onDown: function () {
      if (this.commandsIndex > -1) {
        this.commandsIndex -= 1;

        if (this.commandsIndex === -1) {
          this.inputCommand = this.savedInputCommand;
        } else {
          this.inputCommand = this.previousCommands[this.commandsIndex];
        }
      }
    },
    onSubmit: function (e) {
      e.preventDefault();
      if (this.inputCommand.length === 0) {
        return;
      }

      if (this.previousCommands[0] !== this.inputCommand) {
        this.previousCommands.unshift(this.inputCommand);
      }

      invoke("run_command", { input: this.inputCommand }).catch((error) =>
        console.error(error)
      );

      this.inputCommand = "";
      this.savedInputCommand = "";
      this.commandsIndex = -1;
    },
    startup: function () {
      if (this.$route.path === "/console") {
        invoke("get_console_texts").then((texts) => {
          texts.forEach((content) => {
            this.consoleTexts.push({ id: this.texts, content });
            this.texts++;
          });

          this.$refs.scroller.scrollToBottom();
        });

        invoke("get_console_input_commands").then((commands) => {
          this.previousCommands = commands;
        });
      }
    },
  },
  computed: {},
  created: function () {
    this.startup();
  },
  watch: {
    // call again the method if the route changes
    $route: "startup",
  },
};
