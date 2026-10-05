module top;
  logic [3:0] v; int m; bit [3:0] k2; logic [3:0] t;
  initial begin
    k2 = 4'd3;
    for (int i = 0; i < 11; i++) begin
      v = i[3:0];
      case (v) inside [k2+4'd5:4'd9]: m = 3; default: m = 0; endcase
      $display("v=%0d m=%0d", v, m);
    end
    #10 $finish;
  end
endmodule
