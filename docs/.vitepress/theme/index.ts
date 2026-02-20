import DefaultTheme from 'vitepress/theme'
import type { Theme } from 'vitepress'
import CardGrid from '../components/CardGrid.vue'
import Card from '../components/Card.vue'
import Steps from '../components/Steps.vue'
import Step from '../components/Step.vue'
import Tabs from '../components/Tabs.vue'
import Tab from '../components/Tab.vue'
import InDuskHeader from '../components/InDuskHeader.vue'
import FullscreenDiagram from '../components/FullscreenDiagram.vue'
import './custom.css'

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.component('CardGrid', CardGrid)
    app.component('Card', Card)
    app.component('Steps', Steps)
    app.component('Step', Step)
    app.component('Tabs', Tabs)
    app.component('Tab', Tab)
    app.component('InDuskHeader', InDuskHeader)
    app.component('FullscreenDiagram', FullscreenDiagram)
  },
} satisfies Theme
