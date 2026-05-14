import { expect, test } from 'vitest';
import { Player } from '../renderer/DataModel/Player';

test('answerType01', () => {
  expect(Player.getPlayerNameFromId('Player_John Smith_1234')).toBe('John Smith');
});

test('getPlayerNameFromId_underscoresInName', () => {
  expect(Player.getPlayerNameFromId('Player_John_Smith_1234')).toBe('John_Smith');
});

test('getPlayerNameFromId_embeddedDigitsInName', () => {
  expect(Player.getPlayerNameFromId('Player_Team 123_1234')).toBe('Team 123');
});

test('parseYear_postSenior', () => {
  const p = new Player('Test');
  p.yearString = '5Sr.';
  expect(p.year).toBe(17);
  p.yearString = '5 Sr.';
  expect(p.year).toBe(17);
  p.yearString = '5sr';
  expect(p.year).toBe(17);
});
