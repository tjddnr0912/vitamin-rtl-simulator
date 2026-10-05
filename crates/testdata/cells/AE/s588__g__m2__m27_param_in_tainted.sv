module top;
  parameter W = 4'd2;
  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    if (t[3:2] == W) f = 4'd1; else if ((t & 4'b0000) == 4'd0) f = 4'd2; else f = 4'd3;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #2 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
