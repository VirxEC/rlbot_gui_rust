const invoke = window.__TAURI__.invoke;

export default {
	name: 'console',
	template: /*html*/`
	<div class="overflow-auto flex-grow-1">
	<b-navbar class="navbar">
		<b-navbar-brand>
			<img class="logo" src="imgs/rlbot_logo.png">
			<span class="rlbot-brand" style="flex: 1">RLBot Pre-Launch</span>
		</b-navbar-brand>
		<b-navbar-nav class="ml-auto">
			<b-spinner v-if="showProgressSpinner" variant="success" label="Spinning" class="mr-2"></b-spinner>
			<b-button @click="$router.push('/console')" variant="dark" class="ml-2">
				Console
			</b-button>
		</b-navbar-nav>
	</b-navbar>
	<b-container fluid>
		<b-card no-body class="bot-pool p-1">
			<b-card title="Python configuration menu" id="python-setup" hide-footer centered>
				<b-form>
					<p class="mr-3">Path to Python executable or command (verison 3.7.X for best compatibility):</p>
					<b-form-input :placeholder="rec_python ? 'Confused? Click the button just below!' : ''" id="python-exe-path" v-model="python_path" size="md" width="100%"></b-form-input>
					<b-button v-if="noPython && !rec_python && is_windows" variant="success" class="mt-3" @click="installPython()"><b-icon icon="exclamation-triangle-fill"/>&nbsp;Download & Install Python</b-button>
					<b-button v-if="rec_python && rec_python != python_path" variant="success" class="mt-3" @click="partialPythonSetup()"><b-icon icon="exclamation-triangle-fill"/>&nbsp;Use recommended Python path</b-button>
					<hr>
					<p class="mr-3">RLBot <b>requires</b> some basic Python packages to be installed in order to run <b>that you do not have.</b></p>
					<p class="mr-3">Clicking "Apply" will attempt to <b>install, repair, and/or update</b> these packages.</p>
					<div style="display:flex; align-items: flex-end">
						<b-button variant="primary" class="mt-3" @click="applyPythonSetup()">Apply</b-button>
					</div>
				</b-form>
			</b-card>

			<b-toast id="snackbar-toast" v-model="showSnackbar" toaster="b-toaster-bottom-center" body-class="d-none">
				<template v-slot:toast-title>
					{{snackbarContent}}
				</template>
			</b-toast>
		</b-card>
	</b-container>
	</div>
	`,
	components: {},
	data () {
		return {
			showSnackbar: false,
			snackbarContent: null,
			showProgressSpinner: false,
			noPython: false,
			hasRLBot: false,
			python_path: "",
			rec_python: null,
			is_windows: false,
		}
	},
	methods: {
		installPython: function() {
			this.showProgressSpinner = true;
			invoke("install_python").then(result => {
				this.snackbarContent = result != null ? "Successfully installed Python to your system, installing required packages" : "Uh-oh! An error happened somewhere!";
				this.showSnackbar = true;

				if (result != null) {
					invoke("install_basic_packages").then((result) => {
						let message = result.exit_code === 0 ? 'Successfully installed ' : 'Failed to install ';
						message += result.packages.join(", ");
						if (result.exit_code != 0) {
							message += " with exit code " + result.exit_code;
						}
						this.snackbarContent = message;
						this.showSnackbar = true;
						this.showProgressSpinner = false;
	
						this.startup();
					});
				} else {
					this.showProgressSpinner = false;
				}
			});
		},
		applyPythonSetup: function () {
			this.showProgressSpinner = true;

			if (this.rec_python && !this.python_path) {
				this.python_path = this.rec_python;
			}

			invoke("set_python_path", { path: this.python_path }).then(() => {
				invoke("install_basic_packages").then((result) => {
					let message = result.exit_code === 0 ? 'Successfully installed ' : 'Failed to install ';
					message += result.packages.join(", ");
					if (result.exit_code != 0) {
						message += " with exit code " + result.exit_code;
					}
					this.snackbarContent = message;
					this.showSnackbar = true;
					this.showProgressSpinner = false;

					this.startup();
				});
			});
		},
		partialPythonSetup: function () {
			this.python_path = this.rec_python;
			invoke("set_python_path", { path: this.python_path }).then(() => {
				this.startup();
			});
		},
		startup: function() {
			invoke("check_rlbot_python").then(support => {
				this.noPython = !support.python;
				this.hasRLBot = support.rlbotpython;

				if (!this.noPython && this.hasRLBot) {
					this.$router.replace('/');
					return
				}

				this.startup_inner();
			});
		},
		startup_inner: function() {
			invoke("get_python_path").then(path => this.python_path = path);
			invoke("get_detected_python_path").then(path => {console.log(path);this.rec_python = path});

			invoke("is_windows").then(is_windows => this.is_windows = is_windows);
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
