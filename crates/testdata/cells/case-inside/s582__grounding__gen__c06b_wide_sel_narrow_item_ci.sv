module top;
  logic [7:0] v; int m;
  initial begin
    v = 8'h0C; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      [4'd1:4'd2]: m = 2;
      default: m = 0;
    endcase
    $display("v=%h m=%0d", v, m);
    v = 8'h1C; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      [4'd1:4'd2]: m = 2;
      default: m = 0;
    endcase
    $display("v=%h m=%0d", v, m);
    v = 8'h08; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      [4'd1:4'd2]: m = 2;
      default: m = 0;
    endcase
    $display("v=%h m=%0d", v, m);
    v = 8'h01; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      [4'd1:4'd2]: m = 2;
      default: m = 0;
    endcase
    $display("v=%h m=%0d", v, m);
    v = 8'h11; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      [4'd1:4'd2]: m = 2;
      default: m = 0;
    endcase
    $display("v=%h m=%0d", v, m);
    v = 8'hF8; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      [4'd1:4'd2]: m = 2;
      default: m = 0;
    endcase
    $display("v=%h m=%0d", v, m);
    #10 $finish;
  end
endmodule
