module top;
  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    t[1:0] = 2'b11;
    if (a == 1) t = 4'd10;
    f = t;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
endmodule
