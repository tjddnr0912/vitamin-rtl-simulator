module child #(parameter int W = cf(2)) (input logic [W:0] p);
  function automatic int cf(input int a);
    cf = 1;
    if (a == 1) cf = 10;
  endfunction
  if (cf(2) == 1) begin : gy
    initial #1 $display("%m W=%0d b=%0d p=%h gy", W, $bits(p), p);
  end
endmodule
module top;
  logic [7:0] bus = 8'hA5;
  child #(.W(3)) u[1:0] (.p(bus));
  child #(.W(1)) v[3:0] (.p(bus));
  initial #5 $finish;
endmodule
