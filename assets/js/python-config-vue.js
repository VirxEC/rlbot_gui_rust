import MiniConsole from "./mini-console-vue.js";

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
	<br>
	<b-container fluid="xl">
		<b-card no-body>
			<b-card title="Python configuration menu" id="python-setup" hide-footer centered class="text-center">
				<b-form v-if="advanced || !is_windows">
					<h6 class="mr-3">Path to python.exe or Python command - verison 3.7.X for best compatibility; virtual environment python.exe's also work:</h6>
					<b-form-input class="text-center" style="width: 100%" :placeholder="rec_python || (is_rec_isolated && is_windows) ? 'Confused? Click the button just below!' : 'Path to python.exe'" id="python-exe-path" v-model="python_path" size="md"></b-form-input>
					<span v-if="!is_rec_isolated && is_windows"><b-button variant="success" class="mt-3" @click="installPython()">&nbsp;Install custom isolated Python 3.7</b-button><br></span>
					<b-button v-if="rec_python && rec_python != python_path" :variant="(!is_rec_isolated && is_windows) ? 'warning' : 'success'" class="mt-3" @click="partialPythonSetup()"><b-icon v-if="!is_rec_isolated && is_windows" icon="exclamation-triangle-fill"/>&nbsp;Use found Python path</b-button>
					<span v-if="python_path != '' && !showProgressSpinner">
						<hr>
						<p class="mr-3">RLBot <b>requires</b> some basic Python packages to be installed in order to run <b>that you do not have.</b></p>
						<p class="mr-3">Clicking "Apply" will attempt to <b>install, repair, and/or update</b> these packages.</p>
						<b-button variant="primary" class="mt-3" @click="applyPythonSetup()">Apply</b-button>
					</span>
				</b-form>
				<b-form v-else>
					<span v-if="is_rec_isolated">
						<p class="mr-3">RLBot <b>requires</b> some basic Python packages to be installed in order to run <b>that you do not have.</b></p>
						<b-button variant="success" class="mt-3" @click="applyPythonSetup()">Install, repair, and/or update required packages</b-button>
					</span>
					<span v-else>
						<p class="mr-3">RLBot <b>requires</b> Python as well as some basic packages in order to run.</p>
						<b-button variant="success" class="mt-3" @click="installPython()">Install isolated Python and required packages</b-button>
					</span>
					<b-button variant="warning" class="mt-3" @click="advanced = true">Advanced settings</b-button>
				</b-form>
			</b-card>

			<b-toast id="snackbar-toast" v-model="showSnackbar" toaster="b-toaster-bottom-center" body-class="d-none">
				<template v-slot:toast-title>
					{{snackbarContent}}
				</template>
			</b-toast>

			<b-modal size="xl" class="overflow-auto" title="Installing required packages..." id="install-console" hide-footer centered>
				<mini-console/>
			</b-modal>

			<b-modal id="download-modal" v-bind:title="downloadModalTitle" hide-footer centered no-close-on-backdrop no-close-on-esc hide-header-close>
				<div class="text-center">
					<b-icon icon="cloud-download" font-scale="3"></b-icon>
				</div>
				<b-progress variant="success" :value="downloadProgressPercent" animated class="mt-2 mb-2"></b-progress>
				<p>{{ downloadStatus }}</p>
			</b-modal>
		</b-card>
	</b-container>
	</div>
	`,
	components: {
		'mini-console': MiniConsole
	},
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
			is_rec_isolated: false,
			advanced: false,
			downloadProgressPercent: 0,
			downloadStatus: '',
			updateDownloadProgressPercent: listen("update-download-progress", event => {
				this.downloadProgressPercent = event.payload.percent;
				this.downloadStatus = event.payload.status;
			}),
		}
	},
	methods: {
		installPython: function() {
			this.showProgressSpinner = true;
			this.downloadStatus = "Starting Isolated Python 3.7 installation...";
			this.downloadProgressPercent = 0;
			this.$bvModal.show("download-modal");
			
			invoke("install_python").then(result => {
				this.$bvModal.hide("download-modal");
				this.snackbarContent = result != null ? "Successfully installed Python to your system, installing required packages" : "Uh-oh! An error happened somewhere!";
				this.showSnackbar = true;

				if (result != null) {
					this.$bvModal.show("install-console");
					invoke("install_basic_packages").then((result) => {
						let message = result.exit_code === 0 ? 'Successfully installed ' : 'Failed to install ';
						message += result.packages.join(", ");
						if (result.exit_code != 0) {
							message += " with exit code " + result.exit_code;
						}
						this.snackbarContent = message;
						this.showSnackbar = true;
	
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
				invoke("check_rlbot_python").then(support => {
					if (support.python && !support.rlbotpython) {
						this.$bvModal.show("install-console");
						invoke("install_basic_packages").then((result) => {
							let message = result.exit_code === 0 ? 'Successfully installed ' : 'Failed to install ';
							message += result.packages.join(", ");
							if (result.exit_code != 0) {
								message += " with exit code " + result.exit_code;
							}
							this.snackbarContent = message;
							this.showSnackbar = true;

							this.startup(false);
						});
					} else {
						this.startup();
					}
				});
			});
		},
		partialPythonSetup: function () {
			this.showProgressSpinner = true;
			this.python_path = this.rec_python;
			invoke("set_python_path", { path: this.python_path }).then(() => {
				this.startup();
			});
		},
		startup: function(redirect_check_for_updates=true) {
			invoke("check_rlbot_python").then(support => {
				this.noPython = !support.python;
				this.hasRLBot = support.rlbotpython;
				
				if (!this.noPython && this.hasRLBot) {
					this.$router.replace(`/?check_for_updates=${redirect_check_for_updates}`);
					return
				}
				
				this.startup_inner();
			});
		},
		startup_inner: function() {
			this.$bvModal.hide("install-console");
			this.showProgressSpinner = false;
			invoke("get_python_path").then(path => this.python_path = path);
			invoke("get_detected_python_path").then(info => {
				this.rec_python = info[0];
				this.is_rec_isolated = info[1];
				console.log(this.rec_python);
				console.log(this.is_rec_isolated);
			});

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
