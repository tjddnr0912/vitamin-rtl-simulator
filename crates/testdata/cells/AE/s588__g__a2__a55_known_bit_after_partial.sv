module top;
  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    t[0] = 1'b1;
    f = {3'b000, t[0]};
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
