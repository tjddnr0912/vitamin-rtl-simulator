localparam int W = 4;
typedef logic [W-1:0] v_t;
module top;
  localparam int W = 8;
  v_t x;
  initial begin x = '1; #1 $display("x=%h b=%0d", x, $bits(x)); $finish; end
endmodule
