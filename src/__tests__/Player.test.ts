import { expect, test } from 'vitest';
import { Player } from '../renderer/DataModel/Player';

test('answerType01', () => {
  expect(Player.getPlayerNameFromId('Player_John Smith_1234')).toBe('John Smith');
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
