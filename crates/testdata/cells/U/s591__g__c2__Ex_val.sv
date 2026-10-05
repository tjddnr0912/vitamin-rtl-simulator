module top;
  typedef enum {E0, E1} e_t;
  localparam int K = E1;
  localparam [64:0] KW = E1;
  wire [64:0] wv = E1;
  initial begin
    #1 $display("val E1=%0d K=%0d KW=%0d wv=%0d b=%0d", E1, K, KW, wv, $bits(E1));
    case (E1) 0: $display("pc zero"); 1: $display("pc one"); default: $display("pc def"); endcase
  end
  initial #100 $finish;
endmodule
