module top;
  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    integer i;
    if (a == 1) t = 4'd5;
    if (t) f = 4'd1; else f = 4'd2;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
