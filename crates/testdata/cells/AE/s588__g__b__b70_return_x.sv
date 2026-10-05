module top;
  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    integer i;
    if (a == 1) t = 4'd5;
    return t;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
