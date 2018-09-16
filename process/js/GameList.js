var React = require('react');

class GameList extends React.Component{

  constructor(props) {
    super(props);
    this.addGame = this.addGame.bind(this);
  }

  //tell the mainInterface to open a New Game form
  addGame () {
    this.props.openModal();
  }

  selectedString() {
    if(this.props.numberSelected == 0) { return ''; }
    return this.props.numberSelected + ' selected';
  }

  //add the disabled class if the limit on the number of games has been reached
  //or if you don't yet have two teams to make a game with
  addBtnDisabled() {
    if(this.props.gameList.length > 2000 || this.props.numberOfTeams < 2) {
      return 'disabled';
    }
    return '';
  }

  render () {
    if (this.props.whichPaneActive != 'gamesPane') {
      return null;
    }
    if(this.props.gameList.length == 0) {
      var message;
      if(this.props.numberOfTeams < 2) {
        message = 'Add more teams to get started';
      }
      else if(this.props.totalGames == 0) {
        message = 'Add a game to get started';
      }
      else { //there are games, but they've all been excluded based on the search
        message = 'Your search did not match any games'
      }
      return (
        <div className="zero-state-container">
          <div className="qb-zero-state">
            <img src="banana-bunch.png" width="80" height="55"/><br/><br/>
            <h6>{message}</h6>
          </div>
          <div className="fixed-action-btn btn-floating-div">
            <button className={'btn-floating btn-large green tooltipped ' + this.addBtnDisabled()} data-position="left" data-tooltip="Add a game" onClick={this.addGame}>
              <i className="large material-icons">add</i>
            </button>
          </div>
        </div>
      );
    }
    return(
      <div className="container">
        <div className="sort-buttons">
          {this.selectedString()}
        </div>
        <ul className="collection">{this.props.gameList}</ul>
        <div className="fixed-action-btn btn-floating-div">
          <button className="btn-floating btn-large green tooltipped" data-position="left" data-tooltip="Add a game" onClick={this.addGame}>
            <i className="large material-icons">add</i>
          </button>
        </div>
      </div>
    )
  }

}

module.exports=GameList;
