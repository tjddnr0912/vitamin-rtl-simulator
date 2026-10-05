module top;
  function automatic int f(input int a);
    logic [3:0] t;
    logic [3:0] c;
    integer i;
    c = 4'b1010;
    if (a == 1) t = 4'd5;
    f = t ? 4'd1 : 4'd2;
  endfunction
  localparam int P = f(2);
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #100 $finish;
endmodule
