import Main from "./main-vue.js";
import Sandbox from "./sandbox-vue.js";
import Story from "./story-mode.js";
import Console from "./console-vue.js";
import PythonConfig from "./python-config-vue.js";

const routes = [
  { path: "/", component: Main },
  { path: "/console", component: Console },
  { path: "/sandbox", component: Sandbox },
  { path: "/story", component: Story },
  { path: "/python-config", component: PythonConfig },
];

// eslint-disable-next-line no-undef
const router = new VueRouter({
  routes,
});

// eslint-disable-next-line no-undef
const store = new Vuex.Store({
  state: {
    activeBot: null,
  },
  mutations: {
    setActiveBot(state, bot) {
      state.activeBot = bot;
    },
  },
});

// eslint-disable-next-line no-undef, no-unused-vars
const app = new Vue({
  router,
  store,
  el: "#app",
  data: function () {
    return {
      bodyStyle: null,
    };
  },
  methods: {
    changeBackgroundImage: function (bodyStyle) {
      this.bodyStyle = bodyStyle;
    },
  },
});

// We loaded this javascript successfully, so get rid of the help message that suggests the new launcher script,
// because they either don't need it or already have it.
document.getElementById("javascript-trouble").remove();
