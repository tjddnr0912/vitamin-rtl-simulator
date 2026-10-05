module top;
  function automatic logic [3:0] f(input int a);
    bit [3:0] t;
    f = 4'd7;
    if (a == 1) f = 4'd10;
    f = f + t;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
