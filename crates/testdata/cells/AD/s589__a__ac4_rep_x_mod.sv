module top;
  function automatic logic [3:0] hm(input int x);
    logic [3:0] t;
    hm = t;
  endfunction
  wire [31:0] w;
  assign w = {4'd0, {hm(2){1'b1}}};
  initial begin #1 $display("w=%h", w); $finish; end
  initial #50 $finish;
endmodule
