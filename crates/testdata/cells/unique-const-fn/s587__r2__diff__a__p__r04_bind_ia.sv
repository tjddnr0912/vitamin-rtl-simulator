module chk #(parameter int W = cf(2)) (input logic [W:0] q);
  function automatic int cf(input int a);
    cf = 3;
    if (a == 1) cf = 10;
  endfunction
  initial #1 $display("%m W=%0d b=%0d q=%h", W, $bits(q), q);
endmodule
module child (input logic [3:0] p);
  function automatic int cf(input int a);
    cf = 7;
  endfunction
endmodule
module top;
  logic [7:0] bus = 8'hA5;
  child u[1:0] (.p(bus));
  bind child chk c (.q(p));
  initial #5 $finish;
endmodule
