import AppearanceEditor from './appearance-editor-vue.js'
import MutatorField from './mutator-field-vue.js'
import BotCard from './bot-card-vue.js'
import ScriptCard from './script-card-vue.js'
import BotPool from './bot-pool-vue.js'
import TeamCard from './team-card-vue.js'
import LauncherPreferenceModal from './launcher-preference-vue.js'

const invoke = window.__TAURI__.invoke;

const HUMAN = {'name': 'Human', 'type_': 'human', 'image': 'imgs/human.png'};
const STARTING_BOT_POOL = [
	HUMAN,
	{'name': 'Psyonix Allstar', 'type_': 'psyonix', 'skill': 1, 'image': 'imgs/psyonix.png' },
	{'name': 'Psyonix Pro', 'type_': 'psyonix', 'skill': 0.5, 'image': 'imgs/psyonix.png' },
	{'name': 'Psyonix Rookie', 'type_': 'psyonix', 'skill': 0, 'image': 'imgs/psyonix.png' }
];

export default {
	name: 'match-setup',
	template: /*html*/`
	<div class="noscroll-flex flex-grow-1">
	<b-navbar class="navbar">
		<b-navbar-brand>
			<img class="logo" src="imgs/rlbot_logo.png">
			<span class="rlbot-brand" style="flex: 1">RLBot</span>
		</b-navbar-brand>


		<b-navbar-nav class="ml-auto">
			<b-spinner v-if="showProgressSpinner" variant="success" label="Spinning" class="mr-2"></b-spinner>
			<b-button @click="$router.replace('/console')" variant="dark" class="ml-2">
				Console
			</b-button>
			<span id="sandbox-button-wrapper">
				<b-button
					@click="$router.replace('/sandbox')" variant="dark" class="ml-2"
					:disabled="!matchSettings.enable_state_setting">
					State Setting Sandbox
				</b-button>
			</span>
			<b-tooltip target="sandbox-button-wrapper" v-if="!matchSettings.enable_state_setting">
				<b-icon class="warning-icon" icon="exclamation-triangle-fill"></b-icon>
				State setting is turned off, sandbox won't work!
			</b-tooltip>

			<b-button 
				@click="$router.replace('/story')" variant="dark" class="ml-2"
				>
				Story Mode
			</b-button>

			<b-dropdown right class="ml-4" variant="dark">
				<template v-slot:button-content>
					Menu
				</template>

				<b-dropdown-item @click="pythonSetup()">
					Edit/Repair Python Settings
				</b-dropdown-item>
				<b-dropdown-item @click="downloadBotPack()">
					Repair bot pack
				</b-dropdown-item>
				<b-dropdown-item v-b-modal.package-installer>
					Install missing python package
				</b-dropdown-item>
				<b-dropdown-item @click="resetMatchSettingsToDefault()">
					Reset match settings
				</b-dropdown-item>
				<b-dropdown-item @click="pickAndEditAppearanceFile()">
					Edit appearance config file
				</b-dropdown-item>
			</b-dropdown>
		</b-navbar-nav>
	</b-navbar>
	<b-container fluid class="rlbot-main-config noscroll-flex flex-grow-1">

	

	<b-modal title="Install Package" id="package-installer" centered>

		<b-form-group label="Package Name" label-for="package-name">
			<b-form-input id="package-name" v-model="packageString"></b-form-input>
		</b-form-group>

		<template v-slot:modal-footer>
			<b-button @click="installPackage()" variant="primary">Install Package</b-button>
		</template>
	</b-modal>


		<b-card no-body class="bot-pool noscroll-flex flex-grow-1">
			<div class="center-flex my-2">
				<span class="rlbot-card-header ml-1">Bots</span>
				<b-dropdown class="ml-2 mr-2">
					<template v-slot:button-content><b-icon icon="plus"/>Add</template>
					<b-dropdown-item  @click="updateBotPack()">
						<b-icon icon="cloud-download"></b-icon>
						<span>Download Bot Pack</span>
					</b-dropdown-item>
					<b-dropdown-item v-b-modal.new-bot-modal>
						<b-icon icon="pencil-square"></b-icon>
						<span>Start Your Own Bot!</span>
					</b-dropdown-item>
					<b-dropdown-item @click="updateMapPack()">
						<b-icon icon="geo"></b-icon>
						<span>Download Maps</span>
					</b-dropdown-item>
					<b-dropdown-item  @click="pickBotFolder()">
						<b-icon icon="folder-plus"></b-icon>
						<span>Load Folder</span>
					</b-dropdown-item>
					<b-dropdown-item @click="pickBotConfig()">
						<b-icon icon="file-earmark-plus"></b-icon>
						<span>Load Cfg File</span>
					</b-dropdown-item>
				</b-dropdown>
				<b-button @click="prepareFolderSettingsDialog" v-b-modal.folder-settings-modal>
				<b-icon icon="gear"/> Manage bot folders
				</b-button>
				<b-button v-b-modal.recommendations-modal class="ml-2" v-if="recommendations">
					<b-icon icon="hand-thumbs-up"/> Recommendations
				</b-button>
			</div>

			<bot-pool
				:bots="botPool"
				:scripts="scriptPool"
				@bot-clicked="addToTeam($event, teamSelection)"
				ref="botPool"
				:display-human="displayHumanInBotPool"
				class="noscroll-flex"
			/>

		</b-card>

		<b-row>
			<b-col @click="teamSelection = 'blue'">
				<team-card v-model="blueTeam" team-class="blu" @botadded="handleBotAddedToTeam">
					<b-form-radio v-model="teamSelection" name="team-radios" value="blue">Add to Blue Team</b-form-radio>
				</team-card>
			</b-col>

			<b-col @click="teamSelection = 'orange'">
				<team-card v-model="orangeTeam" team-class="org" @botadded="handleBotAddedToTeam">
					<b-form-radio v-model="teamSelection" name="team-radios" value="orange">Add to Orange Team</b-form-radio>
				</team-card>
			</b-col>
		</b-row>

		<b-card v-if="matchOptions" class="settings-card">
			<span class="rlbot-card-header">Match Settings</span>
			<div style="display:flex; align-items: flex-end">

				<div style="max-width: 250px">
					<label for="map_selection">Map</label>
					<b-form-select v-model="matchSettings.map" id="map_selection" @change="updateBGImage(matchSettings.map)">
						<b-form-select-option v-for="map in matchOptions.map_types" :key="map" v-bind:value="map">
							{{map}}
							<span v-if="map == 'BeckwithPark_Midnight'">(DO NOT SELECT)</span> <!-- temporary until https://github.com/RLBot/RLBot/issues/523 is fixed -->
						</b-form-select-option>
					</b-form-select>
				</div>
				<div class="ml-2">
					<label for="mode_selection">Mode</label>
					<b-form-select v-model="matchSettings.game_mode" id="mode_selection">
						<b-form-select-option v-for="mode in matchOptions.game_modes" :key="mode" v-bind:value="mode">{{mode}}</b-form-select-option>
					</b-form-select>
				</div>

				<b-button class="ml-4" v-b-modal.mutators-modal>
					Mutators
					<b-badge variant="info" v-if="activeMutatorCount > 0">{{ activeMutatorCount }}</b-badge>
				</b-button>
				<b-button class="ml-2" v-b-modal.extra-modal>Extra</b-button>
				<b-button class="ml-2" v-b-modal.launcher-modal>
					<img class="platform-icon" src="imgs/steam.png" /> /
					<img class="platform-icon" src="imgs/epic.png" />
				</b-button>

				<span style="flex-grow: 1"></span>

				<b-button @click="noPython || !hasRLBot ? pythonSetup() : startMatch()" :variant="noPython || !hasRLBot ? 'warning' : 'success'" size="lg" :disabled="matchStarting" class="start-match-btn" style="margin-top: -10px;">
					<span v-if="noPython">Invalid path to Python</span>
					<span v-else-if="!hasRLBot">Basic packages not detected</span>
					<span v-else-if="matchStarting">Starting match</span>
					<span v-else-if="gameAlreadyLaunched">Start another match</span>
					<span v-else>Launch Rocket League<br>and start match</span>
				</b-button>
				<b-button @click="killBots()" variant="secondary" size="lg" class="ml-2">Stop</b-button>
			</div>

			<b-modal title="Python configuration menu" id="python-setup" hide-footer centered>
				<b-form>
					<p class="mr-3">Path to Python executable or command:</p>
					<b-form-input id="python-exe-path" v-model="python_path" size="md" width="100%"></b-form-input>
					<b-button v-if="noPython && rec_python" variant="success" class="mt-3" @click="python_path = rec_python"><b-icon icon="exclamation-triangle-fill"/>&nbsp;Insert recommended Python path</b-button>
					<hr>
					<p class="mr-3">RLBot requires some basic Python packages to be installed in order to run.</p>
					<p class="mr-3">Clicking apply will attempt to install and/or update these packages.</p>
				</b-form>

				<div style="display:flex; align-items: flex-end">
					<b-button variant="primary" class="mt-3" @click="applyPythonSetup()">Apply</b-button>
					&nbsp;
					<b-button variant="warning" class="mt-3" @click="partialPythonSetup()">Apply without configuring</b-button>
				</div>
			</b-modal>

			<div>
				<b-form-checkbox v-model="matchSettings.randomizeMap" class="mt-1 mb-1">
					Randomize Map
				</b-form-checkbox>
			</div>

			<b-modal title="Extra Options" id="extra-modal" size="md" hide-footer centered>
				<div><b-form-checkbox v-model="matchSettings.enable_rendering">Enable Rendering (bots can draw on screen)</b-form-checkbox></div>
				<div><b-form-checkbox v-model="matchSettings.enable_state_setting">Enable State Setting (bots can teleport)</b-form-checkbox></div>
				<div><b-form-checkbox v-model="matchSettings.auto_save_replay">Auto Save Replay</b-form-checkbox></div>
				<div><b-form-checkbox v-model="matchSettings.skip_replays">Skip Replays</b-form-checkbox></div>
				<div><b-form-checkbox v-model="matchSettings.instant_start">Instant Start</b-form-checkbox></div>
				<div><b-form-checkbox v-model="matchSettings.enable_lockstep">Enable Lockstep</b-form-checkbox></div>
				<mutator-field label="Existing Match Behaviour" :options="matchOptions.match_behaviours" v-model="matchSettings.match_behavior" class="mt-3"></mutator-field>
			</b-modal>

			<b-modal id="mutators-modal" title="Mutators" size="lg" hide-footer centered>

				<b-row>
					<b-col>
						<mutator-field label="Match Length" :options="matchOptions.mutators.match_length_types" v-model="matchSettings.mutators.match_length"></mutator-field>
						<mutator-field label="Max Score" :options="matchOptions.mutators.max_score_types" v-model="matchSettings.mutators.max_score"></mutator-field>
						<mutator-field label="Overtime Type" :options="matchOptions.mutators.overtime_types" v-model="matchSettings.mutators.overtime"></mutator-field>
						<mutator-field label="Game Speed" :options="matchOptions.mutators.game_speed_types" v-model="matchSettings.mutators.game_speed"></mutator-field>
						<mutator-field label="Respawn Time" :options="matchOptions.mutators.respawn_time_types" v-model="matchSettings.mutators.respawn_time"></mutator-field>
					</b-col>
					<b-col>
						<mutator-field label="Max Ball Speed" :options="matchOptions.mutators.ball_max_speed_types" v-model="matchSettings.mutators.ball_max_speed"></mutator-field>
						<mutator-field label="Ball Type" :options="matchOptions.mutators.ball_type_types" v-model="matchSettings.mutators.ball_type"></mutator-field>
						<mutator-field label="Ball Weight" :options="matchOptions.mutators.ball_weight_types" v-model="matchSettings.mutators.ball_weight"></mutator-field>
						<mutator-field label="Ball Size" :options="matchOptions.mutators.ball_size_types" v-model="matchSettings.mutators.ball_size"></mutator-field>
						<mutator-field label="Ball Bounciness" :options="matchOptions.mutators.ball_bounciness_types" v-model="matchSettings.mutators.ball_bounciness"></mutator-field>
					</b-col>
					<b-col>
						<mutator-field label="Boost Amount" :options="matchOptions.mutators.boost_amount_types" v-model="matchSettings.mutators.boost_amount"></mutator-field>
						<mutator-field label="Rumble Type" :options="matchOptions.mutators.rumble_types" v-model="matchSettings.mutators.rumble"></mutator-field>
						<mutator-field label="Boost Strength" :options="matchOptions.mutators.boost_strength_types" v-model="matchSettings.mutators.boost_strength"></mutator-field>
						<mutator-field label="Gravity" :options="matchOptions.mutators.gravity_types" v-model="matchSettings.mutators.gravity"></mutator-field>
						<mutator-field label="Demolition" :options="matchOptions.mutators.demolish_types" v-model="matchSettings.mutators.demolish"></mutator-field>
					</b-col>
				</b-row>


				<b-button @click="resetMutatorsToDefault()">Reset Defaults</b-button>

			</b-modal>
		</b-card>

		<b-toast id="snackbar-toast" v-model="showSnackbar" toaster="b-toaster-bottom-center" body-class="d-none">
			<template v-slot:toast-title>
				{{snackbarContent}}
		    </template>
		</b-toast>

		<b-toast id="bot-pack-available-toast" v-model="showBotpackUpdateSnackbar" title="Bot Pack Update Available!" toaster="b-toaster-bottom-center">
			<b-button variant="primary" @click="updateBotPack()" style="margin-left: auto;">Download</b-button>
		</b-toast>

		<b-modal id="bot-info-modal" size="xl" :title="activeBot.name" v-if="activeBot && activeBot.info" hide-footer centered>

			<img v-if="activeBot.logo" class="bot-logo" v-bind:src="activeBot.logo">
			<p><span class="bot-info-key">Developers:</span> {{activeBot.info.developer}}</p>
			<p><span class="bot-info-key">Description:</span> {{activeBot.info.description}}</p>
			<p><span class="bot-info-key">Fun Fact:</span> {{activeBot.info.fun_fact}}</p>
			<p><span class="bot-info-key">GitHub:</span>
				<a :href="activeBot.info.github" target="_blank">{{activeBot.info.github}}</a></p>
			<p><span class="bot-info-key">Language:</span> {{activeBot.info.language}}</p>
			<p>
				<span class="bot-info-key">Tags:</span>
				<b-badge v-for="tag in activeBot.info.tags" class="ml-1">{{tag}}</b-badge>
			</p>
			<p class="bot-file-path">{{activeBot.path}}</p>

			<div>
				<b-button v-if="activeBot.type !== 'script'" @click="showAppearanceEditor(activeBot.looks_path)" v-b-modal.appearance-editor-dialog>
					<b-icon icon="card-image"></b-icon> Edit Appearance
				</b-button>
				<b-button v-if="activeBot.path" @click="showPathInExplorer(activeBot.path)">
					<b-icon icon="folder"></b-icon> Show Files
				</b-button>
			</div>
		</b-modal>

		<b-modal id="language-warning-modal" title="Compatibility Warning" hide-footer centered>
			<div v-if="activeBot && activeBot.warn">
				<div v-if="activeBot.warn === 'java'">
					<p><b>{{activeBot.name}}</b> requires Java and it looks like you don't have it installed!</p>
					To play with it, you'll need to:
					<ol>
						<li>Download Java from <a href="https://java.com" target="_blank">java.com</a></li>
						<li>Install it</li>
						<li>Make sure you've <a href="https://javatutorial.net/set-java-home-windows-10">set the JAVA_HOME environment variable</a></li>
						<li>Reboot your computer</li>
					</ol>
				</div>
				<div v-if="activeBot.warn === 'chrome'">
					<p>
						This bot requires Google Chrome for its auto-run feature, and it looks like
						you don't have it installed! You can
						<a href="https://www.google.com/chrome/" target="_blank">download it here</a>.
					</p>
				</div>
				<div v-if="activeBot.warn === 'pythonpkg'">
					<p>
						This bot needs some python package versions you haven't installed yet:
						<code><span v-for="missing in activeBot.missing_python_packages">{{missing}} </span></code>
					</p>
					<b-button @click="installRequirements(activeBot.path)"
							variant="primary">Install Now</b-button>
					<p v-if="!languageSupport.fullpython">
						If the installation fails, try downloading our <a href="https://github.com/RLBot/RLBotGUI/releases/download/v1.0/RLBotGUI.msi">new launcher script</a>
						which makes RLBotGUI better with package management.
					</p>
				</div>
				<div v-if="activeBot.warn === 'node'">
					<p><b>{{activeBot.name}}</b> requires Node.js to run javascript and it looks like you don't have it installed!</p>
					To play with it, you'll need to:
					<ol>
						<li>Download Node.js from <a href="https://nodejs.org/" target="_blank">nodejs.org</a></li>
						<li>Install it</li>
						<li>Restart RLBotGUI</li>
					</ol>
				</div>
			</div>
		</b-modal>

		<b-modal id="new-bot-modal" title="Create New Bot" hide-footer centered>
			<b-form inline>
				<label class="mr-3">Bot Name</label>
				<b-form-input v-model="newBotName"></b-form-input>
			</b-form>
			<div>
				<b-form-group>
					<b-form-radio v-model="newBotLanguageChoice" name="lang-radios" value="python">Python</b-form-radio>
					<b-form-radio v-model="newBotLanguageChoice" name="lang-radios" value="python_hive">Python Hivemind</b-form-radio>
					<b-form-radio v-model="newBotLanguageChoice" name="lang-radios" value="rust">Rust</b-form-radio>
					<b-form-radio v-model="newBotLanguageChoice" name="lang-radios" value="scratch">Scratch</b-form-radio>
				</b-form-group>
			</div>
			<p style="margin-top: -1rem">If your language isn't listed here, try <a href="https://github.com/RLBot/RLBot/wiki/Supported-Programming-Languages" target="_blank">this list</a>.</p>

			<b-button variant="primary" @click="beginNewBot(newBotLanguageChoice, newBotName)">Begin</b-button>
		</b-modal>

		<b-modal id="folder-settings-modal" title="Folder Settings" size="xl" hide-footer centered>
			<span v-for="settingsList in [folderSettings.folders, folderSettings.files]">
				<b-form inline v-for="(settings, path) in settingsList">
					<b-form-checkbox switch v-model="settings.visible" class="folder-setting-switch">
						{{ path }}
					</b-form-checkbox>

					<b-button size="sm" class="icon-button" @click="showPathInExplorer(path)" variant="outline-info" v-b-tooltip.hover title="Open folder in explorer">
						<b-icon icon="folder"></b-icon>
					</b-button>

					<b-button size="sm" variant="outline-danger" class="icon-button" @click="Vue.delete(settingsList, path)">
						<b-icon icon="x"></b-icon>
					</b-button>
				</b-form>
			</span>
			<b-button variant="primary" class="mt-3" @click="applyFolderSettings()">Apply</b-button>

		</b-modal>

		<b-modal id="download-modal" v-bind:title="downloadModalTitle" hide-footer centered no-close-on-backdrop no-close-on-esc hide-header-close>
			<div class="text-center">
				<b-icon icon="cloud-download" font-scale="3"></b-icon>
			</div>
			<b-progress variant="success" :value="downloadProgressPercent" animated class="mt-2 mb-2"></b-progress>
			<p>{{ downloadStatus }}</p>
		</b-modal>

		<b-modal id="recommendations-modal" size="lg" hide-footer centered title="Recommendations" v-if="recommendations">
			<p>Not sure which bots to play against? Try our recommended picks:</p>
			<b-list-group>
				<b-list-group-item v-for="recommendation in recommendations.recommendations">
					<bot-card v-for="bot in recommendation.bots" :bot="bot" :draggable="false" hidewarning />
					<b-button variant="primary" class="float-right" @click="selectRecommendation(recommendation.bots)">Select</b-button>
				</b-list-group-item>
			</b-list-group>
		</b-modal>

		<b-modal id="no-rlbot-flag-modal" title="Error while starting match" centered>
			<p>This is probably due to Rocket League not being started by RLBot. Please close Rocket League and let RLBot open it for you. Do not start Rocket League yourself.<br /><br />If this message still appears, try restarting RLBot.</p>
			<template v-slot:modal-footer>
				<b-button @click="startMatch({'blue': blueTeam, 'orange': orangeTeam});$bvModal.hide('no-rlbot-flag-modal')" >Retry</b-button>
				<b-button @click="$bvModal.hide('no-rlbot-flag-modal')" variant="primary">OK</b-button>
			</template>
		</b-modal>

		<appearance-editor
				v-bind:active-bot="activeBot"
				v-bind:path="appearancePath"
				v-bind:map="matchSettings.map"
				id="appearance-editor-dialog" />
				
		<b-modal title="Preferred Rocket League Launcher" id="launcher-modal" size="md" hide-footer centered>
			<launcher-preference-modal modal-id="launcher-modal" />
		</b-modal>

	</div>

	</b-container>
	</div>
	`,
	components: {
		'appearance-editor': AppearanceEditor,
		'mutator-field': MutatorField,
		'bot-card': BotCard,
		'script-card': ScriptCard,
		'bot-pool': BotPool,
		'team-card': TeamCard,
		'launcher-preference-modal': LauncherPreferenceModal,
	},
	data () {
		return {
			botPool: STARTING_BOT_POOL,
			scriptPool: [],
			blueTeam: [HUMAN],
			orangeTeam: [],
			teamSelection: 'orange',
			matchOptions: null,
			matchSettings: {
				map: null,
				game_mode: null,
				skip_replays: false,
				instant_start: false,
				enable_lockstep: false,
				match_behavior: null,
				mutators: {
					match_length: null,
					max_score: null,
					overtime: null,
					series_length: null,
					game_speed: null,
					ball_max_speed: null,
					ball_type: null,
					ball_weight: null,
					ball_size: null,
					ball_bounciness: null,
					boost_amount: null,
					rumble: null,
					boost_strength: null,
					gravity: null,
					demolish: null,
					respawn_time: null
				},
				randomizeMap: false,
				enable_rendering: false,
				enable_state_setting: false,
				auto_save_replay: false,
				scripts: [],
			},
			randomMapPool: [],
			packageString: null,
			showSnackbar: false,
			snackbarContent: null,
			showProgressSpinner: false,
			gameAlreadyLaunched: false,
			matchStarting: false,
			languageSupport: null,
			newBotName: '',
			newBotLanguageChoice: 'python',
			folderSettings: {
				files: {},
				folders: {}
			},
			isBotpackUpToDate: true,
			downloadProgressPercent: 0,
			downloadStatus: '',
			showBotpackUpdateSnackbar: false,
			appearancePath: '',
			recommendations: null,
			downloadModalTitle: "Downloading Bot Pack",
			noPython: false,
			hasRLBot: false,
			python_path: "",
			rec_python: null,
			init: false,
		}
	},

	methods: {
		pythonSetup: function(event)  {
			invoke("get_detected_python_path").then((path) => this.rec_python = path);
			this.$bvModal.show("python-setup")
		},
		quickReloadWarnings: function() {
			invoke("get_language_support").then(support => {
				this.languageSupport = support;
				this.noPython = !this.languageSupport.python;
				this.hasRLBot = this.languageSupport.rlbotpython;
			});

			for (let i = STARTING_BOT_POOL.length; i < this.botPool.length; i++) {
				this.botPool[i].missing_python_packages = null;
			}

			invoke("get_missing_bot_packages", { bots: this.botPool.slice(STARTING_BOT_POOL.length) }).then(bots => {
				bots.forEach(packageInfo => {
					let index = packageInfo.index + STARTING_BOT_POOL.length;
					this.botPool[index].warn = packageInfo.warn;
					this.botPool[index].missing_python_packages = packageInfo.missing_packages;
				});
			});

			for (let i = 0; i < this.blueTeam.length; i++) {
				this.blueTeam[i].missing_python_packages = null;
			}

			invoke("get_missing_bot_packages", { bots: this.blueTeam }).then(bots => {
				bots.forEach(packageInfo => {
					this.blueTeam[packageInfo.index].warn = packageInfo.warn;
					this.blueTeam[packageInfo.index].missing_python_packages = packageInfo.missing_packages;
				});
			});

			for (let i = 0; i < this.orangeTeam.length; i++) {
				this.orangeTeam[i].missing_python_packages = null;
			}

			invoke("get_missing_bot_packages", { bots: this.orangeTeam }).then(bots => {
				bots.forEach(packageInfo => {
					this.orangeTeam[packageInfo.index].warn = packageInfo.warn;
					this.orangeTeam[packageInfo.index].missing_python_packages = packageInfo.missing_packages;
				});
			});

			for (let i = 0; i < this.scriptPool.length; i++) {
				this.scriptPool[i].missing_python_packages = null;
			}

			invoke("get_missing_script_packages", { scripts: this.scriptPool }).then(scripts => {
				scripts.forEach(packageInfo => {
					this.scriptPool[packageInfo.index].warn = packageInfo.warn;
					this.scriptPool[packageInfo.index].missing_python_packages = packageInfo.missing_packages;
				});
			});
		},
		applyPythonSetup: function() {
			this.showProgressSpinner = true;
			invoke("set_python_path", { path: this.python_path }).then(() => {
				this.$bvModal.hide("python-setup");
				invoke("install_basic_packages").then((result) => {
					let message = result.exit_code === 0 ? 'Successfully installed ' : 'Failed to install ';
					message += result.packages.join(", ");
					if (result.exit_code != 0) {
						message += " with exit code " + result.exit_code;
					}
					this.snackbarContent = message;
					this.showSnackbar = true;
					this.showProgressSpinner = false;
					
					this.quickReloadWarnings();
				});
			});
		},
		partialPythonSetup: function() {
			invoke("set_python_path", { path: this.python_path }).then(() => {
				this.$bvModal.hide("python-setup");
				this.quickReloadWarnings();
			});
		},
		startMatch: async function (event) {
			this.matchStarting = true;

			if (this.matchSettings.randomizeMap) await this.setRandomMap();

			this.matchSettings.scripts = this.scriptPool.filter((val) => { return val.enabled });
			invoke("save_match_settings", { settings: this.matchSettings });
			invoke("save_team_settings", { blueTeam: this.blueTeam, orangeTeam: this.orangeTeam });

			const blueBots = this.blueTeam.map((bot) => { return  {'name': bot.name, 'team': 0, 'type': bot.type, 'skill': bot.skill, 'path': bot.path} });
			const orangeBots = this.orangeTeam.map((bot) => { return  {'name': bot.name, 'team': 1, 'type': bot.type, 'skill': bot.skill, 'path': bot.path} });

			// start match asynchronously, so it doesn't block things like updating the background image
			// setTimeout(() => {
			// 	eel.start_match(blueBots.concat(orangeBots), this.matchSettings);
			// }, 0);
		},
		killBots: function(event) {
			// eel.kill_bots();
			this.matchStarting = false;
		},
		pickBotFolder: function (event) {
			invoke("pick_bot_folder").then(() => {
				invoke("get_folder_settings").then(this.folderSettingsReceived);
			});
		},
		pickBotConfig: function (event) {
			invoke("pick_bot_config").then(() => {
				invoke("get_folder_settings").then(this.folderSettingsReceived);
			});
		},
		addToTeam: function(bot, team) {
			if (team === 'orange') {
				this.orangeTeam.push(bot);
			} else {
				this.blueTeam.push(bot);
			}
			this.handleBotAddedToTeam(bot);
		},
		handleBotAddedToTeam: function(bot) {
            if (bot.warn) {
                this.$store.commit('setActiveBot', bot);
                this.$bvModal.show('language-warning-modal');
            }
        },
		setRandomMap: async function() {
			if (this.randomMapPool.length == 0) {
				let response = await fetch("json/standard-maps.json");
				this.randomMapPool = await response.json();
			}

			let randomMapIndex = Math.floor(Math.random() * this.randomMapPool.length);
			this.matchSettings.map = this.randomMapPool.splice(randomMapIndex, 1)[0];
			this.updateBGImage(this.matchSettings.map);
		},
		resetMutatorsToDefault: function() {
			const self = this;
			Object.keys(this.matchOptions.mutators).forEach(function (mutator) {
				const mutatorName = mutator.replace('_types', '');
				self.matchSettings.mutators[mutatorName] = self.matchOptions.mutators[mutator][0];
			});
		},
		resetMatchSettingsToDefault: function() {
			this.matchSettings.map = this.matchOptions.map_types[0];
			this.matchSettings.game_mode = this.matchOptions.game_modes[0];
			this.matchSettings.match_behavior = this.matchOptions.match_behaviours[0];
			this.matchSettings.skip_replays = false;
			this.matchSettings.instant_start = false;
			this.matchSettings.enable_lockstep = false;
			this.matchSettings.randomizeMap = false;
			this.matchSettings.enable_rendering = false;
			this.matchSettings.enable_state_setting = true;
			this.matchSettings.auto_save_replay = false;
			this.matchSettings.scripts = [];
			this.resetMutatorsToDefault();

			this.updateBGImage(this.matchSettings.map);
		},
		updateBGImage: function(mapName) {
			let bodyStyle = {backgroundImage: 'url(../imgs/arenas/' + mapName + '.jpg), url(../imgs/arenas/UtopiaRetro.jpg)'};
			this.$emit('background-change', bodyStyle);
		},
		downloadBotPack: function() {
			this.showBotpackUpdateSnackbar = false;
			this.downloadModalTitle = "Downloading Bot Pack" 
			this.$bvModal.show('download-modal');
			this.downloadStatus = "Starting";
			this.downloadProgressPercent = 0;
			// eel.download_bot_pack()(this.botPackUpdated.bind(this, 'Downloaded Bot Pack!'));
		},
		updateBotPack: function() {
			this.showBotpackUpdateSnackbar = false;
			this.downloadModalTitle = "Updating Bot Pack" 
			this.$bvModal.show('download-modal');
			this.downloadStatus = "Starting";
			this.downloadProgressPercent = 0;
			// eel.update_bot_pack()(this.botPackUpdated.bind(this, 'Updated Bot Pack!'));
		},
		updateMapPack: function() {
			this.showBotpackUpdateSnackbar = false;
			this.downloadModalTitle = "Downloading Custom Maps" 
			this.$bvModal.show('download-modal');
			this.downloadStatus = "Starting";
			this.downloadProgressPercent = 0;
			// eel.update_map_pack()(this.botPackUpdated.bind(this, 'Downloaded Maps!'));
		},
		showAppearanceEditor: function(looksPath) {
			this.appearancePath = looksPath;
			this.appearancePath = looksPath;
			this.$bvModal.show('appearance-editor-dialog');
		},
		pickAndEditAppearanceFile: function() {
			invoke("pick_appearance_file").then((path) => {
				this.$store.commit('setActiveBot', null);
				if (path) this.showAppearanceEditor(path);
			});
		},
		showPathInExplorer: function (path) {
			invoke("show_path_in_explorer", { path: path });
		},
		beginNewBot: function (language, bot_name) {
			if (!bot_name) {
				this.snackbarContent = "Please choose a proper name!";
				this.showSnackbar = true;
			} else if (language === 'python') {
				this.showProgressSpinner = true;
				invoke("begin_python_bot", { botName: bot_name }).then(this.botLoadHandler);
			} else if (language === 'scratch') {
				this.showProgressSpinner = true;
				invoke("begin_scratch_bot", { botName: bot_name }).then(this.botLoadHandler);
			} else if (language === 'python_hive') {
				this.showProgressSpinner = true;
				invoke("begin_python_hivemind", { hiveName: bot_name }).then(this.botLoadHandler);
			} else if (language === 'rust') {
				this.showProgressSpinner = true;
				invoke("begin_rust_bot", { botName: bot_name }).then(this.botLoadHandler);
			}
		},
		prepareFolderSettingsDialog: function() {
			invoke("get_folder_settings").then(this.folderSettingsReceived);
		},
		applyFolderSettings: async function() {
			invoke("save_folder_settings", { botFolderSettings: this.folderSettings }).then(() => {
				this.botPool = STARTING_BOT_POOL;
				this.scriptPool = [];
				invoke("scan_for_bots").then(this.botsReceived);
				invoke("scan_for_scripts").then(this.scriptsReceived);
			});
		},
		botLoadHandler: function (response) {
			this.$bvModal.hide('new-bot-modal');
			this.showProgressSpinner = false;
			if (response.error) {
				this.snackbarContent = response.error;
				this.showSnackbar = true;
			} else {
				this.folderSettings["files"][response.bot.name] = response.bot;
				this.botsReceived([response.bot]);
			}
		},
		botsReceived: function (bots) {
			const freshBots = bots.filter( (bot) =>
					!this.botPool.find( (element) => element.path === bot.path ));
			freshBots.forEach((bot) => bot.warn = null);

			this.applyLanguageWarnings(freshBots);
			let botPool = this.botPool.slice(STARTING_BOT_POOL.length).concat(freshBots);
			botPool = botPool.sort((a, b) => a.name.localeCompare(b.name));
			this.botPool = STARTING_BOT_POOL.concat(botPool);
			this.distinguishDuplicateBots(this.botPool);
			this.showProgressSpinner = false;

			invoke("get_missing_bot_packages", { bots: this.botPool.slice(STARTING_BOT_POOL.length) }).then(botPackageInfos => {
				botPackageInfos.forEach(botPackageInfo => {
					let index = botPackageInfo.index + STARTING_BOT_POOL.length;
					this.botPool[index].warn = botPackageInfo.warn;
					this.botPool[index].missing_python_packages = botPackageInfo.missing_packages;
				});
			});

			invoke("get_missing_bot_logos", { bots: this.botPool.slice(STARTING_BOT_POOL.length) }).then(botLogos => {
				botLogos.forEach(botLogo => {
					let index = botLogo.index + STARTING_BOT_POOL.length;
					this.botPool[index].logo = botLogo.logo;
				});
			});
		},

		scriptsReceived: function (scripts) {
			const freshScripts = scripts.filter( (script) =>
					!this.scriptPool.find( (element) => element.path === script.path ));
			freshScripts.forEach((script) => {script.enabled = !!this.matchSettings.scripts.find( (element) => element.path === script.path )});

			this.applyLanguageWarnings(freshScripts);
			this.scriptPool = this.scriptPool.concat(freshScripts).sort((a, b) => a.name.localeCompare(b.name));
			this.distinguishDuplicateBots(this.scriptPool);
			this.showProgressSpinner = false;

			invoke("get_missing_script_packages", { scripts: this.scriptPool }).then(scriptPackageInfos => {
				scriptPackageInfos.forEach(scriptPackageInfo => {
					this.scriptPool[scriptPackageInfo.index].warn = scriptPackageInfo.warn;
					this.scriptPool[scriptPackageInfo.index].missing_python_packages = scriptPackageInfo.missing_packages;
				});
			});

			invoke("get_missing_script_logos", { scripts: this.scriptPool }).then(scriptLogos => {
				scriptLogos.forEach(scriptLogo => {
					this.scriptPool[scriptLogo.index].logo = scriptLogo.logo;
				});
			});
		},
		applyLanguageWarnings: function (bots) {
			if (this.languageSupport) {
				bots.forEach((bot) => {
					if (bot.info && bot.info.language) {
						const language = bot.info.language.toLowerCase();
						if (!this.languageSupport.java && language.match(/java|kotlin|scala/) && !language.match(/javascript/)) {
							bot.warn = 'java';
						}
						if (!this.languageSupport.chrome && language.match(/scratch/)) {
							bot.warn = 'chrome';
						}
						if (!this.languageSupport.node && language.match(/(java|type|coffee)script|js|ts|node/)) {
							bot.warn = 'node';
						}
					}
				});
			}
		},

		distinguishDuplicateBots: function(pool) {
			const uniqueNames = [...new Set(pool.filter(bot => bot.path).map(bot => bot.name))];
			const splitPath = bot => bot.path.split(/[\\|\/]/).reverse();

			for (const name of uniqueNames) {
				const bots = pool.filter(bot => bot.name == name);
				if (bots.length == 1) {
					bots[0].uniquePathSegment = null;
					continue;
				}
				for (let i = 0; bots.length > 0 && i < 99; i++) {
					const pathSegments = bots.map(b => splitPath(b)[i]);
					for (const bot of bots.slice()) {
						const path = splitPath(bot);
						const count = pathSegments.filter(s => s == path[i]).length;
						if (count == 1) {
							bot.uniquePathSegment = path[i];
							bots.splice(bots.indexOf(bot), 1);
						}
					}
				}
			}
		},

		matchOptionsReceived: function(matchOptions) {
			this.matchOptions = matchOptions;
		},

		matchSettingsReceived: function (matchSettings) {
			if (matchSettings) {
				Object.assign(this.matchSettings, matchSettings);
				this.updateBGImage(this.matchSettings.map);
				this.scriptPool.forEach((script) => {script.enabled = !!this.matchSettings.scripts.find( (element) => element.path === script.path )});
			} else {
				this.resetMatchSettingsToDefault();
			}
		},
		teamSettingsReceived: function (teamSettings) {
			if (teamSettings) {
				this.blueTeam = teamSettings.blue_team;
				this.orangeTeam = teamSettings.orange_team;
			}

			invoke("get_missing_bot_packages", { bots: this.blueTeam }).then(botPackageInfos => {
				botPackageInfos.forEach(botPackageInfo => {
					this.blueTeam[botPackageInfo.index].warn = botPackageInfo.warn;
					this.blueTeam[botPackageInfo.index].missing_python_packages = botPackageInfo.missing_packages;
				});
			});

			invoke("get_missing_bot_packages", { bots: this.orangeTeam }).then(botPackageInfos => {
				botPackageInfos.forEach(botPackageInfo => {
					this.orangeTeam[botPackageInfo.index].warn = botPackageInfo.warn;
					this.orangeTeam[botPackageInfo.index].missing_python_packages = botPackageInfo.missing_packages;
				});
			});

			invoke("get_missing_bot_logos", { bots: this.blueTeam }).then(botLogos => {
				botLogos.forEach(botLogo => {
					this.blueTeam[botLogo.index].logo = botLogo.logo;
				});
			});

			invoke("get_missing_bot_logos", { bots: this.orangeTeam }).then(botLogos => {
				botLogos.forEach(botLogo => {
					this.orangeTeam[botLogo.index].logo = botLogo.logo;
				});
			});
			
			this.distinguishDuplicateBots(this.blueTeam.concat(this.orangeTeam));
			this.applyLanguageWarnings(this.blueTeam.concat(this.orangeTeam));
		},

		recommendationsReceived: function (recommendations) {
			if (recommendations) {
				recommendations.recommendations.forEach(recommendation => this.applyLanguageWarnings(recommendation.bots));
				this.recommendations = recommendations;
			}
		},

		folderSettingsReceived: function (folderSettings) {
			this.folderSettings = folderSettings;
			invoke("scan_for_bots").then(this.botsReceived);
			invoke("scan_for_scripts").then(this.scriptsReceived);
			invoke("get_match_options").then(this.matchOptionsReceived);
		},

		botpackUpdateChecked: function (isBotpackUpToDate) {
			this.showBotpackUpdateSnackbar = !isBotpackUpToDate;
			this.isBotpackUpToDate = isBotpackUpToDate;
		},

		botPackUpdated: function (message) {
			this.snackbarContent = message;
			this.showSnackbar = true;
			this.$bvModal.hide('download-modal');
			invoke("get_folder_settings").then(this.folderSettingsReceived);
			invoke("get_recommendations").then(this.recommendationsReceived);
			invoke("get_match_options").then(this.matchOptionsReceived);
			this.$refs.botPool.setDefaultCategory();
			this.isBotpackUpToDate = true;
		},

		onInstallationComplete: function (result) {
			let message = result.exit_code === 0 ? 'Successfully installed ' : 'Failed to install ';
			message += result.packages.join(", ");
			if (result.exit_code != 0) {
				message += " with exit code " + result.exit_code;
			}
			this.snackbarContent = message;
			this.showSnackbar = true;
			this.showProgressSpinner = false;
			
			if (result.exit_code === 0) {
				console.log("hello world");
				this.quickReloadWarnings();
				this.$bvModal.hide('language-warning-modal');
			}
		},
		installPackage: function () {
			this.showProgressSpinner = true;
			invoke("install_package", { packageString: this.packageString }).then(this.onInstallationComplete);
		},
		installRequirements: function (configPath) {
			this.showProgressSpinner = true;
			invoke("install_requirements", { configPath: configPath }).then(this.onInstallationComplete);
		},
		selectRecommendation: function(bots) {
			this.blueTeam = [HUMAN];
			this.orangeTeam = bots.slice();
			bots.forEach(this.handleBotAddedToTeam);
			this.$bvModal.hide('recommendations-modal');
		},
		startup: function() {
			if (this.$route.path != "/" || this.init) {
				return
			}
			invoke('get_folder_settings').then(this.folderSettingsReceived);
			invoke("get_match_options").then(this.matchOptionsReceived);
			invoke("get_match_settings").then(this.matchSettingsReceived);
			invoke("get_team_settings").then(this.teamSettingsReceived);
			invoke("get_python_path").then((path) => this.python_path = path);

			invoke("get_language_support").then((support) => {
				this.languageSupport = support;
				this.noPython = !this.languageSupport.python;
				this.hasRLBot = this.languageSupport.rlbotpython;
				this.applyLanguageWarnings(this.botPool.concat(this.scriptPool));
			});

			// eel.is_botpack_up_to_date()(this.botpackUpdateChecked);
			invoke("get_recommendations").then(this.recommendationsReceived);

			// eel.expose(noRLBotFlagPopup)
			// function noRLBotFlagPopup(title, text){
			// 	this.$bvModal.show("no-rlbot-flag-modal")
			// 	this.matchStarting = false;
			// }

			// eel.expose(matchStarted)
			// function matchStarted(){
			// 	this.matchStarting = false;
			// 	this.gameAlreadyLaunched = true;
			// }
			
			// eel.expose(matchStartFailed)
			// function matchStartFailed(message){
			// 	self.matchStarting = false;
			// 	self.snackbarContent = "Error starting match: " + message + "\n See console for more details.";
			// 	self.showSnackbar = true;
			// }

			// eel.expose(updateDownloadProgress);
			// function updateDownloadProgress(progress, status) {
			// 	this.downloadStatus = status;
			// 	this.downloadProgressPercent = progress;
			// }
			this.init = true;
		},
		allUsableRunnables: function() {
			let runnables = this.botPool.concat(this.scriptPool).concat(this.blueTeam).concat(this.orangeTeam).concat(this.matchSettings.scripts);
			if (this.recommendations) this.recommendations.recommendations.forEach(recommendation => {
				runnables = runnables.concat(recommendation.bots);
			});
			return runnables;
		},
	},
	computed: {
		activeMutatorCount: function() {
			return Object.keys(this.matchSettings.mutators).map(key =>
				this.matchSettings.mutators[key] != this.matchOptions.mutators[key + "_types"][0]
			).filter(Boolean).length;
		},
		activeBot: function() {
			return this.$store.state.activeBot;
		},
		displayHumanInBotPool: function() {
			// only display Human when it's not on any of the teams
			return !this.blueTeam.concat(this.orangeTeam).some(bot => bot.type === "human");
		},
	},
	created: function () {
		this.startup()
	},
	watch: {
		// call again the method if the route changes
		'$route': 'startup'
	},
};
