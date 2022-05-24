import Main from './main-vue.js'
import Sandbox from './sandbox-vue.js'
import Story from './story-mode.js'
import Console from './console-vue.js'

const routes = [
    { path: '/', component: Main },
    { path: '/console', component: Console },
    { path: '/sandbox', component: Sandbox },
    { path: '/story', component: Story }
];

const router = new VueRouter({
    routes: routes
});

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

const app = new Vue({
    router: router,
    store: store,
    el: '#app',
    data: {
        bodyStyle: null
    },
    methods: {
        changeBackgroundImage: function(bodyStyle) {
            this.bodyStyle = bodyStyle;
        }
    }
});

// We loaded this javascript successfully, so get rid of the help message that suggests the new launcher script,
// because they either don't need it or already have it.
document.getElementById("javascript-trouble").remove();
