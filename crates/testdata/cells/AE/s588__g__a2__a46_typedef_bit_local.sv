module top;
  typedef bit [3:0] bt;
  function automatic logic [3:0] f(input int a);
    bt t;
    f = t + 4'd2;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
