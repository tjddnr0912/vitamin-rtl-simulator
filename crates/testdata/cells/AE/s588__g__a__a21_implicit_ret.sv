module top;
  function automatic [3:0] f(input integer a);
    if (a == 1) f = 4'd10;
  endfunction
  localparam [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
