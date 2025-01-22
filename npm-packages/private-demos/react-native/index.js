/**
 * @format
 */

import {AppRegistry} from 'react-native';
import App from './App';
import {name as appName} from './app.json';
// @snippet start reactNativeGetRandomValuesImport
import 'react-native-get-random-values';
// @snippet end reactNativeGetRandomValuesImport

AppRegistry.registerComponent(appName, () => App);
