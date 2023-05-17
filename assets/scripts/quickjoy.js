const QJ_IDLE_COLOR = '#88888844';
const QJ_ACTIVE_COLOR = '#88888866';

class TouchButtonListener {
	trigger(s) {
	}
}

class SingleTouchButton {
	constructor(container, el, label, elListener) {
		this.container = s(container);
		this.el = s(el);
		this.label = label;
		this.state = false;
		this.elListener = elListener;

		var self = this;

		if (this.el != null)
			this.el.textContent = this.label;

		this.container.addEventListener('touchstart', function (e) {
			self.touch(e, true);
		});

		this.container.addEventListener('touchmove', function (e) {
			self.touch(e, true);
		});

		this.container.addEventListener('touchend', function (e) {
			e.preventDefault();
			self.touch(e, false);
		});
	}

	touch(e, pressed) {
		var newState = false;

		if (pressed) {
			newState = true;
		}

		if (newState != this.state) {
			this.state = newState;

			this.elListener.trigger(this.state);

			this.el.style.background = this.state ? QJ_ACTIVE_COLOR : QJ_IDLE_COLOR;
		}
	}
}

class DualTouchButton {
	constructor(container, el1, label1, el2, label2, isHorizontal, elListener) {
		this.container = s(container);
		this.el1 = s(el1);
		this.el2 = s(el2);
		this.label1 = label1;
		this.label2 = label2;
		this.isHorizontal = isHorizontal;
		this.elListener = elListener;
		this.state = 0;

		var self = this;

		if (this.el1 != null)
			this.el1.textContent = this.label1;

		if (this.el2 != null)
			this.el2.textContent = this.label2;

		this.container.addEventListener('touchstart', function (e) {
			self.touch(e, true);
		});

		this.container.addEventListener('touchmove', function (e) {
			self.touch(e, true);
		});

		this.container.addEventListener('touchend', function (e) {
			e.preventDefault();
			self.touch(e, false);
		});
	}

	touch(e, pressed) {
		var newState = 0;

		if (pressed) {
			if (this.isHorizontal) {
				var d = this.container.offsetWidth;
				var v = e.changedTouches[0].clientX;
			} else {
				var d = this.container.offsetHeight;
				var v = e.changedTouches[0].clientY - this.container.getBoundingClientRect().top;
			}

			if (v < (d / 2)) {
				newState = 1;
			} else {
				newState = 2;
			}
		} else {
			newState = 0;
		}

		if (newState != this.state) {
			this.state = newState;
			this.elListener.trigger(this.state);

			this.el1.style.background = this.state == 1 ? QJ_ACTIVE_COLOR : QJ_IDLE_COLOR;
			this.el2.style.background = this.state == 2 ? QJ_ACTIVE_COLOR : QJ_IDLE_COLOR;
		}
	}
}

class SingleTouchButtonJoyListener extends TouchButtonListener {
	constructor(controller, action, callback) {
		super();
		this.controller = controller;
		this.action = action;
		this.callback = callback.bind(callback);
		this.trigger = this.trigger.bind(this);
	}

	trigger(s) {
		this.callback(this.controller, this.action, s);
	}
}

class DualTouchButtonJoyListener extends TouchButtonListener {
	constructor(controller, action1, action2, callback) {
		super();
		this.controller = controller;
		this.action1 = action1;
		this.action2 = action2;
		this.previous = 0;
		this.callback = callback.bind(callback);
		this.trigger = this.trigger.bind(this);
	}

	trigger(s) {
		if (s > 0) {
			this.callback(this.controller, s == 1 ? this.action1 : this.action2, true);
		}
		if (this.previous > 0) {
			this.callback(this.controller, this.previous == 1 ? this.action1 : this.action2, false);
		}
		this.previous = s;
	}
}