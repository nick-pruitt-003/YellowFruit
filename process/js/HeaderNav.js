var React = require('react');

class HeaderNav extends React.Component{

  constructor(props) {
    super(props);
    this.handleSort = this.handleSort.bind(this);
    this.handleOrder = this.handleOrder.bind(this);
    this.handleSearch = this.handleSearch.bind(this);
    this.setPane = this.setPane.bind(this);
    this.openDivModal = this.openDivModal.bind(this);
  }

  handleSort(e) {
    this.props.onReOrder(e.target.id, this.props.orderDir);
  } //handleSort

  handleOrder(e) {
    this.props.onReOrder(this.props.orderBy, e.target.id);
  }

  handleSearch(e) {
    this.props.onSearch(e.target.value);
  }

  setPane(e) {
    this.props.setPane(e.target.id);
  }

  isActive(pane) {
    return this.props.whichPaneActive == pane ? 'active' : '';
  }

  openDivModal() {
    this.props.openDivModal();
  }

  getAssignmentButton() {
    if(this.props.whichPaneActive == 'teamsPane' && this.props.anyTeamSelected) {
      return ( <button className="btn-flat waves-effect yellow-darken-3"
        onClick={this.openDivModal}>Assign Divisions</button> );
    }
    if(this.props.whichPaneActive == 'gamesPane') {
        return ( <button className="btn-flat waves-effect yellow-darken-3">Assign Phases</button> );
    }
    return null;
  }

  render() {
    return(
      <div className="navbar-fixed">
        <nav className="qb-nav">
          <div className="nav-wrapper">
            <ul className="left">
              <li>
                {this.getAssignmentButton()}
              </li>
            </ul>
            <ul id="nav-mobile" className="right">
              <li className={this.isActive("settingsPane")}><a id="settingsPane" onClick={this.setPane}>Settings</a></li>
              <li className={this.isActive("teamsPane")}><a id="teamsPane" onClick={this.setPane}>Teams</a></li>
              <li className={this.isActive("gamesPane")}><a id="gamesPane" onClick={this.setPane}>Games</a></li>
              <li>
                <form>
                  <div className="input-field qb-search">
                    <input id="search" className="qb-search-input" type="search" onChange={this.handleSearch} placeholder="Search" autoFocus type="text" className="form-control" aria-label="Search Appointments" />
                  </div>
                </form>
              </li>
            </ul>
          </div>
        </nav>
      </div>
    ) // return
  }//render
}; //HeaderNav

module.exports = HeaderNav;


/*  TODO: re-implement sorting

    <div className="input-group">
    <div className="input-group-btn">
      <button type="button" className="btn btn-info dropdown-toggle"
        data-toggle="dropdown" aria-haspopup="true" aria-expanded="false">Sort by: <span className="caret"></span></button>
        <ul className="dropdown-menu dropdown-menu-right">
          <li><a href="#" id="teamName" onClick={this.handleSort}>Team Name {(this.props.orderBy === 'teamName') ? <span className="glyphicon glyphicon-ok"></span>:null}</a></li>
          <li><a href="#" id="roster" onClick={this.handleSort}>Roster {(this.props.orderBy === 'roster') ? <span className="glyphicon glyphicon-ok"></span>:null}</a></li>
          <li role="separator" className="divider"></li>
          <li><a href="#" id="asc" onClick={this.handleOrder}>Asc {(this.props.orderDir === 'asc') ? <span className="glyphicon glyphicon-ok"></span>:null}</a></li>
          <li><a href="#" id="desc" onClick={this.handleOrder}>Desc {(this.props.orderDir === 'desc') ? <span className="glyphicon glyphicon-ok"></span>:null}</a></li>
        </ul>
    </div>
  </div> */
