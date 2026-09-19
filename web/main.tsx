import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App, type InitialState } from './app';
import './styles.css';

const root = document.getElementById('root');
if (!root?.dataset.initialState) throw new Error('Initial Todo state is missing');
const state: InitialState = JSON.parse(root.dataset.initialState);
delete root.dataset.initialState;
createRoot(root).render(<StrictMode><App {...state} /></StrictMode>);
