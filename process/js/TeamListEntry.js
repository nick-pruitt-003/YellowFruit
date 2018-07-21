var React = require('react');

class TeamListEntry extends React.Component{

  constructor(props) {
    super(props);
    this.handleDelete = this.handleDelete.bind(this);
    this.selectTeam = this.selectTeam.bind(this);
  }

  handleDelete() {
    this.props.onDelete(this.props.whichItem);
  }

  selectTeam() {
    this.props.onOpenTeam(this.props.whichItem);
  }

  render() {
    return(
      <a className="collection-item">

        <div>
          <div className="team-name">
            {this.props.singleItem.teamName}
            <button className="btn-flat item-edit" title="Edit this team" onClick={this.selectTeam}>
            <i className="material-icons">edit</i></button>
          </div>
          <button className="secondary-content btn-flat item-delete" title="Remove this team" onClick={this.handleDelete}>
          <i className="material-icons">delete</i></button>
          <br/>{this.props.singleItem.roster.join(', ')}
        </div>
      </a>
    )
  }
};

module.exports = TeamListEntry;
