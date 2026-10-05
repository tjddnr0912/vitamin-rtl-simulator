module top;
  logic [3:0] v; int m;
  initial begin
    v = 4'b1000; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      [4'd1:4'd3]: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b0010; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      [4'd1:4'd3]: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b0110; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      [4'd1:4'd3]: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b1100; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      [4'd1:4'd3]: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b0000; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      [4'd1:4'd3]: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b0011; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      [4'd1:4'd3]: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b0100; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      [4'd1:4'd3]: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b0001; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      [4'd1:4'd3]: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    #10 $finish;
  end
endmodule
